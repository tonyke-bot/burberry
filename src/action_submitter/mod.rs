mod map;
mod printer;

#[cfg(feature = "telegram")]
mod telegram;

use std::fmt::Debug;

pub use map::ActionSubmitterMap;
pub use printer::ActionPrinter;

#[cfg(feature = "telegram")]
pub use telegram::TelegramSubmitter;

use tokio::sync::broadcast::Sender;

use crate::ActionSubmitter;

#[derive(Clone)]
pub struct ActionChannelSubmitter<A> {
    sender: Sender<A>,
}

impl<A> ActionChannelSubmitter<A> {
    pub fn new(sender: Sender<A>) -> Self {
        Self { sender }
    }
}

impl<A> ActionSubmitter<A> for ActionChannelSubmitter<A>
where
    A: Send + Sync + Clone + Debug + 'static,
{
    fn submit(&self, action: A) {
        match self.sender.send(action) {
            Ok(_) => (),
            Err(e) => tracing::error!("error submitting action: {:?}", e),
        }
    }
}
