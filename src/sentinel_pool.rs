//! inspired by https://github.com/rust-threadpool/rust-threadpool

use crossbeam::channel::{Receiver, Sender};
use std::sync::Arc;
use std::thread;

#[derive(Clone)]
pub struct SentinelPool {
    inner: Arc<PoolInner>,
}

pub type Job = Box<dyn FnOnce() + Send + 'static>;

impl SentinelPool {
    pub fn new(thread_num: usize) -> Self {
        SentinelPool {
            inner: Arc::new(PoolInner::new(thread_num)),
        }
    }

    pub fn spawn<T: FnOnce() + Send + 'static>(&self, job: T) {
        self.inner.spawn(Box::new(job));
    }
}

struct PoolInner {
    tx: Sender<Job>,
}

impl PoolInner {
    fn new(thread_num: usize) -> Self {
        let (tx, rx) = crossbeam::channel::unbounded::<Job>();
        for _ in 0..thread_num {
            PoolInner::spawn_new_thread(rx.clone());
        }
        PoolInner { tx }
    }

    fn spawn(&self, job: Job) {
        self.tx.send(job).unwrap();
    }

    fn spawn_new_thread(rx: Receiver<Job>) {
        thread::spawn(move || {
            while let Ok(job) = rx.recv() {
                let _sentinel = Sentinel { rx: rx.clone() };
                job();
            }
        });
    }
}

struct Sentinel {
    rx: Receiver<Job>,
}

impl Drop for Sentinel {
    fn drop(&mut self) {
        if thread::panicking() {
            PoolInner::spawn_new_thread(self.rx.clone())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc::channel;

    #[test]
    fn smoke() {
        let pool = SentinelPool::new(1);
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
