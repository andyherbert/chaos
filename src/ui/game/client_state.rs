use crate::data::arena::Arena;
use crate::data::wizard::Wizard;

pub struct ClientState {
    pub wizard: Wizard,
    pub arena: Arena,
    pub turns_left: usize,
}

impl ClientState {
    pub fn new(wizard: Wizard) -> Self {
        Self {
            wizard,
            arena: Arena::new(),
            turns_left: 0,
        }
    }
}
