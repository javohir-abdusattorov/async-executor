## Async executor implementation, running futures concurrently in one thread 

#### Link to the task
Async Rust book:
https://rust-lang.github.io/async-book/02_execution/03_wakeups.html

#### Idea
When implementing asynchronous code in Rust, we can write async functions or blocks. But to execute an async function we need a special executor. Main function itself cannot run async code. To understand how these executors function, I built basic executor myself with the help from book.
This repository contains 2 main parts: *main.rs* - the executor itself, *lib.rs* - example future that only sleeps for a given seconds, to demostrate how in real world async functions will be implemented. In the most cases, we should write async code when we are dependant on I/O, network, or other external source. Writing this way gives us most of concurrency in async code.

#### How it works
**TimerFuture** - is implemented as example future that sleeps for a given duration. ```TimerFuture``` has its shared_state that includes flag ```completed```, and waker from ```std::task::Waker```. waker is key component when building async executors. Because executor should wake-up the future and *continue polling* to complete it. We implemented trait **Future** for ```TimerFuture``` and its single method poll. This method is should be called by executor when it wants to complete the future. When polling the future, executor should provide some type of ```waker``` so that future wakes up executor, when it finishes.
Creating new ```TimerFuture``` we initialize shared_state that is *```Arc<Mutex<SharedState>>```*, and spawn a new thread. Inside the new thread, we sleep the thread. After that mutate the ```completed``` flag, and call ```wake()``` on the waker. In next poll, ```TimerFuture``` looks at shared_state and sees that it is completed and returns *```Poll::Ready```* meaning future as a whole is completed

**Task** - is wrapper for future itself and contains channel which to send message to wake up main executor. Trait **ArcWake** is implemented for ```Task``` and its single method ```wake_by_ref```. This method is automatically called when ```TimerFuture``` wakes up the waker. In this implementation we make clone of ```Task``` self, and send this as a message.

**Spawner** - is simple message sender, which receives future as it self, boxes it to type *```Pin<Box<dyn Future<Output = ...> + Send>>```*. Creates a new ```Task``` and sends the task through channel

**Executor** - struct is part that receives messages from channel and tries to complete future. Runs an infinite loop through ```channel.recv()```, and when receiving message constructs a ```WakerRef``` from ```Task```, it is possible because ```Task``` implements **ArcWake** trait. And creates an ```Context``` from waker. This context is given to future when polling. When receiving *```PollResult```* from poll, if it is still pending ```Executor``` re-attaches future to the ```Task```. And if poll is completed, it just drops all that was used