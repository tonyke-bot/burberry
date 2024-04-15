use crate::{
    ActionSubmitter,
    Executor,
};
use crate::executor::telegram_message::{Message, TelegramMessageDispatcher};

pub struct TelegramSubmitter {
    executor: TelegramMessageDispatcher,
}

impl Default for TelegramSubmitter {
    fn default() -> Self {
        let executor = TelegramMessageDispatcher::default();
        Self { executor }
    }
}

impl ActionSubmitter<Message> for TelegramSubmitter {
    fn submit(&self, action: Message) {
        futures::executor::block_on(async {
            self.executor.execute(action).await.unwrap();
        });
    }
}