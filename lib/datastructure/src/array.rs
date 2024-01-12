use std::ops::Deref;

#[derive(Debug, Clone)]
pub struct CycleArrayVector<T> {
    data: Vec<T>,
    ptr: usize,
}

impl<T> CycleArrayVector<T> {
    pub fn new(data: Vec<T>) -> Self {
        if data.is_empty() {
            panic!("CycleArrayVector does not support empty vector");
        }
        Self { data, ptr: 0 }
    }

    pub fn move_to_next(&mut self) {
        self.ptr = (self.ptr + 1) % self.data.len();
    }
}

impl<T> Deref for CycleArrayVector<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.data[self.ptr]
    }
}
