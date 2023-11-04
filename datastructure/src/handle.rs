use std::{cell::RefCell, rc::Rc};

#[derive(Clone)]
pub struct Handle<T: ?Sized> {
    data: Rc<RefCell<T>>,
}

impl<T> Handle<T> {
    pub fn new(data: T) -> Handle<T> {
        let data = Rc::new(RefCell::new(data));
        Handle { data }
    }

    pub fn operate(&self, f: impl Fn(&T)) {
        let data = self.data.borrow();
        f(&data)
    }

    pub fn operate_mut(&self, f: impl Fn(&mut T)) {
        let mut data = self.data.borrow_mut();
        f(&mut data)
    }
}
