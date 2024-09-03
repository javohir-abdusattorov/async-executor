use std::{
    future::Future,
    pin::Pin,
    sync::{Arc,Mutex},
    task::{Context,Poll,Waker},
    thread,
    time::Duration
};

#[derive(Debug)]
struct SharedState {
    completed: bool,
    waker: Option<Waker>,
}

pub struct TimerFuture {
    shared_state: Arc<Mutex<SharedState>>
}

impl Future for TimerFuture {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        println!("[TimerFuture::poll] context: {:?}", cx);

        let mut shared_state = self.shared_state.lock().unwrap();
        println!("[TimerFuture::poll] shared_state: {:?}", shared_state);

        match shared_state.completed {
            true => {
                Poll::Ready(())
            },
            false => {
                shared_state.waker = Some(cx.waker().clone());
                Poll::Pending
            }
        }
    }
}

impl TimerFuture {
    /// Create a `TimerFuture` which will complete after the provided timeout
    pub fn new(duration: Duration) -> Self {
        let shared_state = Arc::new(Mutex::new(SharedState {
            completed: false,
            waker: None,
        }));

        let thread_shared_state = shared_state.clone();
        thread::spawn(move || {
            thread::sleep(duration);

            let mut shared_state = thread_shared_state.lock().unwrap();
            shared_state.completed = true;

            println!("[TimerFuture::new] shared_state: {:?}", shared_state.waker);
            if let Some(waker) = shared_state.waker.take() {
                // This wakes up the Executor, it receives another task, and starts polling future,
                // We marked TimerFuture's shared_state as completed, so when TimerFuture.poll receives
                // a call, it looks up shared_state and sees that is is completed then returns Poll::Ready
                // result. Receiving this result from poll, Executor finishes its future
                println!("[TimerFuture::new] waking up");
                waker.wake()
            }
        });

        TimerFuture {
            shared_state
        }
    }
}