use futures_core::future::BoxFuture;
use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};
use tokio::runtime::Runtime;

lazy_static::lazy_static! {
    static ref RT :Runtime = gen_rt();
}

fn gen_rt() -> Runtime {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
}

pub fn block_on<T, F>(f: F) -> T
where
    T: Send + 'static,
    F: Future<Output = T> + Send + 'static,
{
    RT.block_on(f)
}

pub enum OneStep<T>
where
    T: Send + 'static,
{
    Working(BoxFuture<'static, T>),
    Done,
}

impl<T> OneStep<T>
where
    T: Send + 'static,
{
    pub fn with_task<F>(task: F) -> Self
    where
        F: Future<Output = T> + Send + 'static,
    {
        OneStep::Working(Box::pin(task))
    }
}

impl<T> Future for OneStep<T>
where
    T: Send + 'static,
{
    type Output = Option<T>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let one_step = self.get_mut();
        match one_step {
            OneStep::Working(future) => match Future::poll(future.as_mut(), cx) {
                Poll::Ready(result) => {
                    *one_step = OneStep::Done;
                    Poll::Ready(Some(result))
                }
                Poll::Pending => Poll::Ready(None),
            },
            OneStep::Done => Poll::Ready(None),
        }
    }
}
