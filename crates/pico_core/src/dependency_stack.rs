use std::cell::RefCell;

#[derive(Debug, Default)]
pub struct DependencyStack<T>(RefCell<Vec<Vec<T>>>);

impl<T> DependencyStack<T> {
    pub fn new() -> Self {
        Self(RefCell::new(Vec::new()))
    }

    pub fn enter(&self) -> DependencyStackGuard<'_, T> {
        self.0.borrow_mut().push(vec![]);
        DependencyStackGuard {
            stack: self,
            released: false,
        }
    }

    pub fn leave(&self) -> Vec<T> {
        self.0
            .borrow_mut()
            .pop()
            .expect("Dependency stack should not be empty. Leave must be called after enter. This indicates a bug in Pico.")
    }

    pub fn push_if_not_empty(&self, make_val: impl FnOnce() -> T) {
        if let Some(entry) = self.0.borrow_mut().last_mut() {
            entry.push(make_val());
        } else {
            // Dependency stack is empty for the outermost memoized function.
            // We don't need to register dependencies for it.
        }
    }
}

pub struct DependencyStackGuard<'a, T> {
    stack: &'a DependencyStack<T>,
    released: bool,
}

impl<T> DependencyStackGuard<'_, T> {
    pub fn release(mut self) -> Vec<T> {
        let dependencies = self.stack.leave();
        self.released = true;
        dependencies
    }
}

impl<T> Drop for DependencyStackGuard<'_, T> {
    fn drop(&mut self) {
        if !self.released {
            self.stack.leave();
        }
    }
}
