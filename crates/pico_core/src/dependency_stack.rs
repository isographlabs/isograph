use std::cell::RefCell;

#[derive(Debug, Default)]
pub struct DependencyStack<T>(RefCell<Vec<Vec<T>>>);

impl<T> DependencyStack<T> {
    pub fn new() -> Self {
        Self(RefCell::new(Vec::new()))
    }

    pub fn enter(&self) {
        self.0.borrow_mut().push(vec![]);
    }

    pub fn leave(&self) -> Vec<T> {
        self.0
            .borrow_mut()
            .pop()
            .expect("Dependency stack should not be empty. Leave must be called after enter. This indicates a bug in Pico.")
    }

    pub fn push_checked(&self, make_val: impl FnOnce() -> T) {
        if let Some(entry) = self.0.borrow_mut().last_mut() {
            entry.push(make_val());
        } else {
            // Dependency stack is empty for the outermost memoized function.
            // We don't need to register dependencies for it.
        }
    }
}
