use std::{
    cell::{Ref, RefCell, RefMut},
    fmt::Debug,
    rc::Rc,
    sync::Arc,
};
use tokio::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};

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
