use futures::{
    future::{BoxFuture, FutureExt},
    task::{waker_ref, ArcWake},
};
use std::{
    future::Future,
    sync::mpsc::{sync_channel, Receiver, SyncSender},
    sync::{Arc, Mutex},
    task::Context,
    time::Duration,
};
use timer_future::TimerFuture;

struct Executor {
    ready_queue: Receiver<Arc<Task>>,
}

impl Executor {
    fn run(&self) {
        let mut task_id = 0;
        while let Ok(task) = self.ready_queue.recv() {
            task_id += 1;
            println!("[Executor::task-{task_id}] received task");

            let mut future_slot = task.future.lock().unwrap();
            if let Some(mut future) = future_slot.take() {
                let waker = waker_ref(&task);
                let context = &mut Context::from_waker(&waker);

                let poll_result = future.as_mut().poll(context);
                println!("[Executor::task-{task_id}] polling future > {:?}", &poll_result);

                if poll_result.is_pending() {
                    *future_slot = Some(future)
                }
            }
        }
    }
}

#[derive(Clone)]
struct Spawner {
    task_sender: SyncSender<Arc<Task>>,
}

impl Spawner {
    fn spawn(&self, future: impl Future<Output = ()> + 'static + Send) {
        let future = future.boxed();
        let task: Arc<Task> = Arc::new(Task {
            future: Mutex::new(Some(future)),
            task_sender: self.task_sender.clone(),
        });

        println!("[Spawner] sending new task to queue");
        self.task_sender.send(task).expect("Too many tasks queued, maximum reached")
    }
}

struct Task {
    future: Mutex<Option<BoxFuture<'static, ()>>>,
    task_sender: SyncSender<Arc<Task>>,
}

impl ArcWake for Task {
    fn wake_by_ref(arc_self: &Arc<Self>) {
        println!("[Task::wake_by_ref] waked up, cloning and sending self to channel");

        let cloned = arc_self.clone();
        arc_self
            .task_sender
            .send(cloned)
            .expect("Too many tasks queued, maximum reached");
    }
}

fn new_executor_and_spawner() -> (Executor, Spawner) {
    const MAX_QUEUED_TASKS: usize = 10_000;
    let (task_sender, ready_queue) = sync_channel(MAX_QUEUED_TASKS);

    (
        Executor { ready_queue },
        Spawner { task_sender },
    )
}

fn main() {
    let (executor, spawner) = new_executor_and_spawner();

    spawner.spawn(async {
        println!("[async] starting future >");

        TimerFuture::new(Duration::from_secs(5)).await;

        println!("[async] future completed <")
    });

    drop(spawner);

    executor.run();
}