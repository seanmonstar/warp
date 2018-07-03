use std::thread;

use crossbeam_channel as crossbeam;
use futures::{Future, Poll};
use futures::sync::oneshot;

use ::never::Never;

/// dox?
pub fn blocking<F, A, R>(threads: usize, blocker: F) -> impl FnClone<A, Blocking<R>>
where
    F: Fn(A) -> R + Clone + Send + 'static,
    A: Send + 'static,
    R: Send + 'static,
{
    blocking_new(threads, move || blocker.clone())
}

/// dox
pub fn blocking_new<F1, F2, A, R>(threads: usize, factory: F1) -> impl FnClone<A, Blocking<R>>
where
    F1: Fn() -> F2 + Clone + Send + 'static,
    F2: Fn(A) -> R,
    A: Send + 'static,
    R: Send + 'static,
{
    assert!(threads > 0, "threads must not be 0");
    let (tx, rx) = crossbeam::unbounded::<(A, oneshot::Sender<R>)>();

    for _ in 0..threads {
        let factory = factory.clone();
        let rx = rx.clone();
        thread::spawn(move || {
            let worker = factory();
            while let Ok((msg, cb)) = rx.recv() {
                let ret = worker(msg);
                let _ = cb.send(ret);
            }
        });
    }


    move |args| {
        let (one_tx, one_rx) = oneshot::channel();
        let _ = tx.send((args, one_tx));
        Blocking {
            i: one_rx,
        }
    }
}

pub struct Blocking<T> {
    i: oneshot::Receiver<T>,
}

impl<T> Future for Blocking<T> {
    type Item = T;
    type Error = Never;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        self.i.poll().map_err(|_| panic!("pool is gone"))
    }
}

pub trait FnClone<A, R>: Fn(A) -> R + Clone {}

impl<F: Fn(A) -> R + Clone, A, R> FnClone<A, R> for F {}

