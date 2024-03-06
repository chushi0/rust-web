use std::future::Future;
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
