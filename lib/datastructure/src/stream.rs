use futures_core::future::BoxFuture;
use futures_core::{Future, Stream};
use std::pin::Pin;
use std::task::{Context, Poll};

pub struct Concurrency<'a, Output> {
    futures: Vec<BoxFuture<'a, Output>>,
}

impl<'a, Output> Concurrency<'a, Output> {
    pub fn new() -> Self {
        Self {
            futures: Vec::new(),
        }
    }

    pub fn submit_task<F>(&mut self, task: F)
    where
        F: Future<Output = Output> + Send + 'a,
    {
        self.futures.push(Box::pin(task));
    }

    pub fn submit_box_task(&mut self, task: Box<dyn Future<Output = Output> + Send + 'a>) {
        self.futures.push(Box::into_pin(task));
    }

    pub fn submit_pin_task(&mut self, task: BoxFuture<'a, Output>) {
        self.futures.push(task);
    }

    #[inline]
    pub fn remain_task(&self) -> usize {
        self.futures.len()
    }
}

impl<'a, Output> Default for Concurrency<'a, Output> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a, Output> Stream for Concurrency<'a, Output> {
    type Item = Output;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if self.futures.is_empty() {
            return Poll::Ready(None);
        }

        let mut finish_task = None;
        for (index, future) in self.futures.iter_mut().enumerate() {
            let result = Future::poll(future.as_mut(), cx);
            if let Poll::Ready(result) = result {
                finish_task = Some((index, result));
                break;
            }
        }

        if let Some((index, result)) = finish_task {
            // this task has been done
            _ = self.futures.remove(index);
            return Poll::Ready(Some(result));
        }

        Poll::Pending
    }
}
