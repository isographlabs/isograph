use prelude::Postfix;

/// A stack whose only mutation is a scoped push: [`pushed`](Stack::pushed)
/// returns a guard that pops on drop, and [`with_pushed`](Stack::with_pushed) scopes
/// the push to a closure. Everything pushed is popped when its scope ends.
#[derive(Debug)]
pub struct Stack<T>(Vec<T>);

impl<T> Stack<T> {
    pub fn new() -> Self {
        Stack(Vec::new())
    }

    /// Every item on the stack, the callers' items included, innermost last.
    pub fn all(&self) -> &[T] {
        self.0.reference()
    }

    /// The item stays until the returned guard drops. The guard borrows the stack, so
    /// it is the only usable handle while it lives; [`stack`](Pushed::stack) hands the
    /// stack back, so a callee still takes `&mut Stack<T>` whether or not its caller
    /// holds a guard.
    pub fn pushed(&mut self, item: T) -> Pushed<'_, T> {
        self.0.push(item);
        Pushed { stack: self }
    }

    /// The `with_` bracketing pattern from iso1's peekable lexer: the item is on the
    /// stack exactly for the duration of the closure.
    pub fn with_pushed<R>(&mut self, item: T, do_stuff: impl FnOnce(&mut Stack<T>) -> R) -> R {
        let mut pushed = self.pushed(item);
        do_stuff(pushed.stack())
    }
}

impl<T> Default for Stack<T> {
    fn default() -> Self {
        Stack::new()
    }
}

/// The guard for one pushed item. Drop pops that item, however the scope exits, a
/// panic included.
#[derive(Debug)]
pub struct Pushed<'a, T> {
    stack: &'a mut Stack<T>,
}

impl<T> Pushed<'_, T> {
    pub fn all(&self) -> &[T] {
        self.stack.all()
    }

    /// The stack, with this guard's item on it.
    pub fn stack(&mut self) -> &mut Stack<T> {
        self.stack
    }
}

impl<T> Drop for Pushed<'_, T> {
    fn drop(&mut self) {
        self.stack.0.pop();
    }
}

#[cfg(test)]
mod test {
    use prelude::Postfix;

    use crate::Stack;

    #[test]
    fn with_pushed_holds_the_item_for_the_closure() {
        let mut stack = Stack::new();

        stack.with_pushed(1, |stack| {
            assert_eq!(stack.all(), [1].reference());
        });

        assert!(stack.all().is_empty());
    }

    #[test]
    fn nested_closures_see_outer_items() {
        let mut stack = Stack::new();

        stack.with_pushed(1, |stack| {
            stack.with_pushed(2, |stack| {
                assert_eq!(stack.all(), [1, 2].reference());
            });
            assert_eq!(stack.all(), [1].reference());
        });
    }

    #[test]
    fn a_pushed_item_pops_when_the_guard_drops() {
        let mut stack = Stack::new();

        {
            let mut pushed = stack.pushed(1);
            assert_eq!(pushed.all(), [1].reference());

            let pushed_again = pushed.stack().pushed(2);
            assert_eq!(pushed_again.all(), [1, 2].reference());
        }

        assert!(stack.all().is_empty());
    }

    #[test]
    fn a_callee_pushes_for_the_rest_of_its_body() {
        fn callee(stack: &mut Stack<i32>) {
            let mut pushed = stack.pushed(2);
            recurse(pushed.stack());
            assert_eq!(pushed.all(), [1, 2].reference());
        }

        fn recurse(stack: &mut Stack<i32>) {
            assert_eq!(stack.all(), [1, 2].reference());
        }

        let mut stack = Stack::new();
        stack.with_pushed(1, |stack| {
            callee(stack);
            assert_eq!(stack.all(), [1].reference());
        });
    }

    #[test]
    fn with_pushed_returns_the_closure_result() {
        let mut stack = Stack::new();

        let result = stack.with_pushed(1, |stack| stack.all().len());

        assert_eq!(result, 1);
    }

    #[test]
    fn a_panic_still_pops() {
        let mut stack = Stack::new();

        let panicked = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            stack.with_pushed(1, |_stack| panic!("unwind out of the closure"));
        }));

        assert!(panicked.is_err());
        assert!(stack.all().is_empty());
    }
}
