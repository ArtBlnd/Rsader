use std::sync::Mutex;

pub struct Flag<T> {
    value: Mutex<Option<T>>,
}

impl<T> Flag<T>
where
    T: Clone,
{
    pub fn new() -> Self {
        Self {
            value: Mutex::new(None),
        }
    }

    pub fn set(&self, value: T) {
        *self.value.lock().unwrap() = Some(value);
    }

    pub fn get(&self) -> Option<T> {
        self.value.lock().unwrap().as_ref().cloned()
    }

    pub fn consume(&self) -> Option<T> {
        self.value.lock().unwrap().take()
    }
}
