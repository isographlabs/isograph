use crate::effect::IsographEffect;
use crate::event::IsographEvent;
use prelude::Postfix;

pub struct IsographState;

impl IsographState {
    #[expect(clippy::unused_self)]
    pub fn handle(&mut self, event: IsographEvent) -> Vec<IsographEffect> {
        match event {
            IsographEvent::HelloWorld => IsographEffect::LogHelloWorld.wrap_vec(),
            IsographEvent::Quit => IsographEffect::Kill.wrap_vec(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::IsographState;
    use crate::effect::IsographEffect;
    use crate::event::IsographEvent;
    use prelude::Postfix;

    #[test]
    fn hello_world_returns_log_hello_world() {
        let mut state = IsographState;
        let effects = state.handle(IsographEvent::HelloWorld);
        assert_eq!(effects, IsographEffect::LogHelloWorld.wrap_vec());
    }

    #[test]
    fn quit_returns_kill() {
        let mut state = IsographState;
        let effects = state.handle(IsographEvent::Quit);
        assert_eq!(effects, IsographEffect::Kill.wrap_vec());
    }
}
