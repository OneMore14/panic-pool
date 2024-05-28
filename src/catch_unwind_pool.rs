use crossbeam::channel::Sender;
use std::panic::{catch_unwind, UnwindSafe};
use std::sync::Arc;
use std::thread::{spawn, JoinHandle};

#[derive(Clone)]
pub struct CatchUnwindPool {
    inner: Arc<PoolInner>,
}

pub type Job = Box<dyn FnOnce() + Send + UnwindSafe + 'static>;

impl CatchUnwindPool {
    pub fn new(thread_num: usize) -> Self {
        CatchUnwindPool {
            inner: Arc::new(PoolInner::new(thread_num)),
        }
    }

    pub fn spawn<T: FnOnce() + Send + UnwindSafe + 'static>(&self, job: T) {
        self.inner.spawn(Box::new(job));
    }
}

struct PoolInner {
    tx: Sender<Job>,
    _threads: Vec<JoinHandle<()>>,
}

impl PoolInner {
    fn new(thread_num: usize) -> Self {
        let (tx, rx) = crossbeam::channel::unbounded::<Job>();
        let threads = (0..thread_num)
            .map(|_| {
                let rx = rx.clone();
                spawn(move || {
                    while let Ok(job) = rx.recv() {
                        // we can use AssertUnwindSafe here, then job does not have to be UnwindSafe
                        let _ = catch_unwind(job);
                    }
                })
            })
            .collect::<Vec<_>>();
        PoolInner {
            tx,
            _threads: threads,
        }
    }

    fn spawn(&self, job: Job) {
        self.tx.send(job).unwrap();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc::channel;

    #[test]
    fn smoke() {
        let pool = CatchUnwindPool::new(1);
        pool.spawn(|| {
            panic!("first panic");
        });
        let (tx, rx) = channel::<i32>();
        pool.spawn(move || {
            tx.send(101).unwrap();
        });
        assert_eq!(rx.recv(), Ok(101));
    }
}
