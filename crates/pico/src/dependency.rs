use std::cell::RefCell;

use crate::{derived_node::DerivedNodeId, epoch::Epoch, u64_types::Key};

#[derive(Debug, Clone, Copy)]
pub struct Dependency {
    pub node_to: NodeKind,
    pub time_verified_or_updated: Epoch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NodeKind {
    Source(Key),
    Derived(DerivedNodeId),
}

#[derive(Debug)]
pub struct TrackedDependencies {
    pub dependencies: Vec<Dependency>,
    pub max_time_updated: Epoch,
}

impl TrackedDependencies {
    pub fn new() -> Self {
        Self {
            dependencies: vec![],
            max_time_updated: Epoch::new(),
        }
    }

    pub fn push(&mut self, dependency: Dependency, time_updated: Epoch) {
        self.max_time_updated = std::cmp::max(time_updated, self.max_time_updated);
        self.dependencies.push(dependency);
    }
}

/// This is a `RefCell` containing a `Vec` of [`TrackedDependencies`] where:
/// - The `Vec` acts as a stack. Each time a memoized function is called,
///   a new `TrackedDependencies` struct is pushed onto the stack.
/// - When a memoized function calls another memoized function or accesses a source,
///   a [`Dependency`] pushed to the current top `TrackedDependencies` struct
///   and updates its `max_time_updated` field.
///
/// `RefCell` gives us dynamically checked borrow checking rules.
/// This is required because calling a memoized function only takes an `&Database`.
#[derive(Debug, Default)]
pub struct DependencyStack(RefCell<Vec<TrackedDependencies>>);

impl DependencyStack {
    pub fn new() -> Self {
        Self(RefCell::new(Vec::new()))
    }

    pub fn enter(&self) -> DependencyStackGuard<'_> {
        eprintln!("enter 1");
        self.0.borrow_mut().push(TrackedDependencies::new());
        eprintln!("enter 2");
        DependencyStackGuard {
            stack: self,
            released: false,
        }
    }

    pub fn leave(&self) -> TrackedDependencies {
        self.0
            .borrow_mut()
            .pop()
            .expect("Dependency stack should not be empty. Leave must be called after enter.")
    }

    pub fn push_if_not_empty(&self, dependency: Dependency, time_updated: Epoch) {
        if let Some(entry) = self.0.borrow_mut().last_mut() {
            entry.push(dependency, time_updated);
        } else {
            // Dependency stack is empty for the outermost memoized function.
            // We don't need to register dependencies for it.
        }
    }
}

pub struct DependencyStackGuard<'a> {
    stack: &'a DependencyStack,
    released: bool,
}

impl DependencyStackGuard<'_> {
    pub fn release(mut self) -> TrackedDependencies {
        let dependencies = self.stack.leave();
        self.released = true;
        dependencies
    }
}

impl Drop for DependencyStackGuard<'_> {
    fn drop(&mut self) {
        if !self.released {
            self.stack.leave();
        }
    }
}
