use freddie_timer::TimerEffect;

use crate::timer::TimerPayload;

pub struct SideEffectToken {
    _not_send: std::marker::PhantomData<*const ()>,
}

impl SideEffectToken {
    pub(crate) fn mint() -> Self {
        Self {
            _not_send: std::marker::PhantomData,
        }
    }
}

pub struct SendLspResponseEffect {
    pub send: Box<dyn FnOnce(&SideEffectToken) + Send>,
}

impl std::fmt::Debug for SendLspResponseEffect {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SendLspResponseEffect")
    }
}

#[derive(Debug)]
pub enum IsographEffect {
    LogHelloWorld,
    Kill,
    SendLspResponse(Box<SendLspResponseEffect>),
    Timer(TimerEffect<TimerPayload>),
}

pub fn no_effects() -> Vec<IsographEffect> {
    Vec::new()
}
