use std::{
    sync::{mpsc, Arc, Mutex},
    thread,
    thread::JoinHandle,
};

pub enum PoolCreationError {
    LessThanOne, // Thread count provided equals 0 or less
}

pub struct ThreadPool {
    workers: Vec<Worker>,
    sender: Option<mpsc::Sender<Job>>,
}

type Job = Box<dyn FnOnce() + Send + 'static>;

impl ThreadPool {
    /// Creates a new ThreadPool.
    /// 
    /// * `thread_count` - Number of threads in the pool.
    ///
    /// # Returns
    /// 
    /// [`ThreadPool`]
    /// 
    /// # Panics
    /// 
    /// The `new` function will panic if the size is zero
    pub fn new(thread_count: usize) -> ThreadPool {
        assert!(thread_count > 0);

        let (sender, receiver) = mpsc::channel();
        let receiver: Arc<Mutex<mpsc::Receiver<Box<dyn FnOnce() + Send>>>> =
            Arc::new(Mutex::new(receiver));
        let mut workers: Vec<Worker> = Vec::with_capacity(thread_count);

        for id in 0..thread_count {
            workers.push(Worker::new(id, Arc::clone(&receiver)));
        }

        ThreadPool {
            workers,
            sender: Some(sender),
        }
    }

    /// Creates a new ThreadPool.
    /// 
    /// * `thread_count` - Number of threads in the pool.
    /// 
    /// # Returns
    /// 
    /// 'Result' type that represents either success ([`Ok(ThreadPool)`]) or  failure ([`Err(PoolCreationError)`])    
    pub fn build(thread_count: usize) -> Result<ThreadPool, PoolCreationError> {
        if thread_count <= 0 {
            return Err(PoolCreationError::LessThanOne);
        } else {
            let (sender, receiver) = mpsc::channel();
            let receiver: Arc<Mutex<mpsc::Receiver<Box<dyn FnOnce() + Send>>>> =
                Arc::new(Mutex::new(receiver));
            let mut workers: Vec<Worker> = Vec::with_capacity(thread_count);

            for id in 0..thread_count {
                workers.push(Worker::new(id, Arc::clone(&receiver)));
            }

            Ok(ThreadPool {
                workers,
                sender: Some(sender),
            })
        }
    }

    pub fn execute<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        let job: Box<F> = Box::new(f);
        self.sender.as_ref().unwrap().send(job).unwrap();
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        drop(self.sender.take());

        for worker in &mut self.workers {
            println!("Shutting down worker {}", worker.id);

            if let Some(thread) = worker.thread.take() {
                thread.join().unwrap();
            }
        }
    }
}

struct Worker {
    id: usize,
    thread: Option<thread::JoinHandle<()>>,
}

impl Worker {
    /// Creates a new Worker.
    /// 
    /// `id` - ID of  the worker.
    /// 
    /// `receiver` - [`mpsc::Receiver<T>`] where `T` is [`Job`]. Receiver wrapped in [`Mutex`] and
    /// [`Arc`] structs.
    ///
    /// # Panics
    ///
    /// This 'new' function will panic. I just don't know on what yet.
    fn new(id: usize, receiver: Arc<Mutex<mpsc::Receiver<Job>>>) -> Worker {
        let thread: JoinHandle<()> = thread::spawn(move || loop {
            let message: Result<Box<dyn FnOnce() + Send>, mpsc::RecvError> =
                receiver.lock().unwrap().recv();

            match message {
                Ok(job) => {
                    println!("Worker {id} got a job; executing.");

                    job();
                }
                Err(_) => {
                    println!("Worker {id} disconnected; shutting down.");
                    break;
                }
            }
        });

        Worker {
            id,
            thread: Some(thread),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_create_threadpool_valid() {
        let threadpool: ThreadPool = ThreadPool::new(4);

        assert_eq!(threadpool.workers.len(), 4);
    }
}
