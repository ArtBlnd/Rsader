use crate::exchange::Exchanges;

pub struct GlobalState {
    pub(super) exchanges: Exchanges,
}

impl GlobalState {
    pub fn exchanges(&self) -> &Exchanges {
        &self.exchanges
    }
}
