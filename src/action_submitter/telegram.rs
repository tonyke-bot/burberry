use std::sync::Arc;

use crate::executor::telegram_message::{Message, TelegramMessageDispatcher};
use crate::ActionSubmitter;

pub struct TelegramSubmitter {
    executor: Arc<TelegramMessageDispatcher>,

    redirect_to: Option<(String, String, Option<String>)>,
}

impl TelegramSubmitter {
    pub fn new_with_redirect(ot_token: String, chat_id: String, thread_id: Option<String>) -> Self {
        let executor = Arc::new(TelegramMessageDispatcher::default());

        Self {
            executor,
            redirect_to: Some((ot_token, chat_id, thread_id)),
        }
    }
}

impl Default for TelegramSubmitter {
    fn default() -> Self {
        let executor = Arc::new(TelegramMessageDispatcher::default());
        Self {
            executor,
            redirect_to: None,
        }
    }
}

impl ActionSubmitter<Message> for TelegramSubmitter {
    fn submit(&self, action: Message) {
        let action = if let Some((bot_token, chat_id, thread_id)) = &self.redirect_to {
            Message {
                bot_token: bot_token.clone(),
                chat_id: chat_id.clone(),
                thread_id: thread_id.clone(),
                ..action
            }
        } else {
            action
        };

        let executor = self.executor.clone();

        std::thread::spawn(move || {
            send_message(executor, action);
        })
        .join()
        .unwrap();
    }
}

#[tokio::main(flavor = "current_thread")]
async fn send_message(executor: Arc<TelegramMessageDispatcher>, action: Message) {
    executor.send_message(action).await;
}
