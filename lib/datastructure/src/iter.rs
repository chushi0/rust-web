use std::future::Future;

pub trait AsyncIter {
    type Item;

    fn async_map<U, F, A>(self, f: F) -> impl Future<Output = impl Iterator<Item = U>> + Send
    where
        U: Send,
        A: Future<Output = U> + Send,
        F: Fn(Self::Item) -> A + Send;

    fn async_for_each<F, A>(self, f: F) -> impl Future<Output = ()>
    where
        A: Future<Output = ()> + Send,
        F: Fn(Self::Item) -> A + Send;
}

impl<T: Iterator<Item = I> + Send, I: Send> AsyncIter for T {
    type Item = I;

    async fn async_map<U, F, A>(self, f: F) -> impl Iterator<Item = U>
    where
        U: Send,
        A: Future<Output = U> + Send,
        F: Fn(Self::Item) -> A + Send,
    {
        let mut await_result = Vec::with_capacity(self.size_hint().0);

        for item in self {
            await_result.push(f(item).await);
        }

        await_result.into_iter()
    }

    async fn async_for_each<F, A>(self, f: F)
    where
        A: Future<Output = ()> + Send,
        F: Fn(Self::Item) -> A + Send,
    {
        for item in self {
            f(item).await;
        }
    }
}
