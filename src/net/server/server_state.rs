use crate::data::arena::Arena;
use crate::data::wizard::ServerWizards;

pub struct ServerState {
    pub wizards: ServerWizards,
    pub arena: Arena,
}
