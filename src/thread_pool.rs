use std::{
    sync::{
        Arc, Mutex,
        mpsc::{self, Receiver, Sender},
    },
    thread,
};

use tracing::{error, info, instrument};

type Job = Box<dyn FnOnce() + Send + 'static>;

#[derive(Debug)]
pub struct ThreadPool {
    workers: Vec<Worker>,
    sender: Sender<Message>,
}

impl ThreadPool {
    pub fn new(thread_count: usize) -> ThreadPool {
        assert!(thread_count > 0, "A positive number of threads must exist");

        let (sender, receiver) = mpsc::channel();
        let receiver: Arc<Mutex<Receiver<Message>>> = Arc::new(Mutex::new(receiver));

        ThreadPool {
            workers: (0..thread_count)
                .map(|i| Worker::new(i, Arc::clone(&receiver)))
                .collect(),
            sender,
        }
    }

    pub fn execute<F: FnOnce() + Send + 'static>(&self, f: F) {
        let job = Box::new(f);
        let _ = self.sender.send(Message::NewJob(job));
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        self.workers.iter().for_each(|_| {
            let _ = self.sender.send(Message::Terminate);
        });

        let workers: Vec<Worker> = std::mem::take(&mut self.workers);

        for worker in workers {
            match worker.spawned_thread.join() {
                Ok(_) => info!("Worker thread shut down successfully"),
                Err(e) => error!(worker_id = worker.id,  error = ?e, "Worker thread panicked"),
            }
        }
    }
}

#[derive(Debug)]
struct Worker {
    id: usize,
    spawned_thread: thread::JoinHandle<()>,
}

impl Worker {
    #[instrument]
    fn new(id: usize, receiver: Arc<Mutex<Receiver<Message>>>) -> Worker {
        Worker {
            id,
            spawned_thread: thread::spawn(move || {
                loop {
                    let res = match receiver.lock() {
                        Ok(locked_mutex) => match locked_mutex.recv() {
                            Ok(msg) => match msg {
                                Message::NewJob(job) => Ok(job),
                                Message::Terminate => break,
                            },
                            Err(e) => {
                                error!(error = ?e, "worked failed to receive job");
                                Err(())
                            }
                        },
                        Err(e) => {
                            error!(mutex = ?receiver, error = ?e, "worker received poisoned mutex");
                            receiver.clear_poison();
                            Err(())
                        }
                    };

                    if let Ok(job) = res {
                        job();
                    }
                }
            }),
        }
    }
}

enum Message {
    NewJob(Job),
    Terminate,
}
