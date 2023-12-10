use std::{
    cell::{Ref, RefCell, RefMut},
    fmt::Debug,
    rc::Rc,
    sync::{mpsc::SendError, Arc},
    time::Duration,
};
use tokio::sync::{
    mpsc::{UnboundedReceiver, UnboundedSender},
    Mutex, RwLock, RwLockReadGuard, RwLockWriteGuard,
};

/// 单线程使用的 Rc<RefCell<T>>
pub struct Handle<T: ?Sized> {
    data: Rc<RefCell<T>>,
}

impl<T> Handle<T> {
    pub fn new(data: T) -> Handle<T> {
        let data = Rc::new(RefCell::new(data));
        Handle { data }
    }

    #[inline]
    pub fn get(&self) -> Ref<'_, T> {
        self.data.borrow()
    }

    #[inline]
    pub fn get_mut(&self) -> RefMut<'_, T> {
        self.data.borrow_mut()
    }
}

impl<T: Debug> Debug for Handle<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Handle").field("data", &self.data).finish()
    }
}

impl<T> Clone for Handle<T> {
    fn clone(&self) -> Self {
        Self {
            data: self.data.clone(),
        }
    }
}

/// 多线程使用的 Arc<RwLock<T>>
///
/// 使用时注意：读写锁不可重入
pub struct SyncHandle<T: ?Sized> {
    data: Arc<RwLock<T>>,
}

impl<T> SyncHandle<T> {
    pub fn new(data: T) -> SyncHandle<T> {
        let data = Arc::new(RwLock::new(data));
        SyncHandle { data }
    }

    #[inline]
    pub async fn get(&self) -> RwLockReadGuard<'_, T> {
        self.data.read().await
    }

    #[inline]
    pub async fn get_mut(&self) -> RwLockWriteGuard<'_, T> {
        self.data.write().await
    }
}

impl<T: Debug> Debug for SyncHandle<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SyncHandle")
            .field("data", &self.data)
            .finish()
    }
}

impl<T> Clone for SyncHandle<T> {
    #[inline]
    fn clone(&self) -> Self {
        Self {
            data: self.data.clone(),
        }
    }
}

pub struct SyncChannel<T: Sized> {
    version: Mutex<u32>,
    sender: Mutex<UnboundedSender<(u32, T)>>,
    receiver: Mutex<UnboundedReceiver<(u32, T)>>,
}

impl<T> SyncChannel<T> {
    pub fn new() -> Self {
        let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();
        Self {
            version: Mutex::new(0),
            sender: Mutex::new(sender),
            receiver: Mutex::new(receiver),
        }
    }

    pub async fn increase_version(&self) {
        (*self.version.lock().await) += 1;
    }

    pub async fn send(&self, data: T) -> Result<(), SendError<T>> {
        let version = *self.version.lock().await;
        self.sender
            .lock()
            .await
            .send((version, data))
            .map_err(|err| SendError(err.0 .1))
    }

    pub async fn recv(&self) -> Option<T> {
        loop {
            let version = *self.version.lock().await;
            match self.receiver.lock().await.recv().await {
                Some((recv_version, data)) => {
                    if recv_version >= version {
                        return Some(data);
                    }
                }
                None => return None,
            }
        }
    }

    pub async fn recv_with_timeout(&self, timeout: Duration) -> Option<T> {
        let recv_result = self.recv();
        let timeout = tokio::time::sleep(timeout);
        tokio::select! {
            res = recv_result => res,
            _ = timeout => None
        }
    }
}
