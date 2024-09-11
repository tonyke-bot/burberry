use async_trait::async_trait;
use reqwest::{Response, StatusCode};
use serde_json::{json, Map};

use crate::types::Executor;

#[derive(Debug, Clone)]
pub struct Message {
    pub bot_token: String,
    pub chat_id: String,
    pub thread_id: Option<String>,
    pub text: String,
    pub disable_notification: Option<bool>,
    pub protect_content: Option<bool>,
    pub disable_link_preview: Option<bool>,
    pub parse_mode: Option<String>,
}

#[derive(Clone, Default)]
pub struct MessageBuilder {
    bot_token: Option<String>,
    chat_id: Option<String>,
    thread_id: Option<String>,
    text: Option<String>,
    disable_notification: Option<bool>,
    protect_content: Option<bool>,
    disable_link_preview: Option<bool>,
    parse_mode: Option<String>,
}

impl MessageBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn bot_token<T: Into<String>>(mut self, bot_token: T) -> Self {
        self.bot_token = Some(bot_token.into());
        self
    }

    pub fn chat_id<T: Into<String>>(mut self, chat_id: T) -> Self {
        self.chat_id = Some(chat_id.into());
        self
    }

    pub fn thread_id<T: Into<String>>(mut self, thread_id: T) -> Self {
        self.thread_id = Some(thread_id.into());
        self
    }

    pub fn text<T: Into<String>>(mut self, text: T) -> Self {
        self.text = Some(text.into());
        self
    }

    pub fn disable_notification(mut self, disable_notification: bool) -> Self {
        self.disable_notification = Some(disable_notification);
        self
    }

    pub fn protect_content(mut self, protect_content: bool) -> Self {
        self.protect_content = Some(protect_content);
        self
    }

    pub fn disable_link_preview(mut self, disable_link_preview: bool) -> Self {
        self.disable_link_preview = Some(disable_link_preview);
        self
    }

    pub fn parse_mode<T: Into<String>>(mut self, parse_mode: T) -> Self {
        self.parse_mode = Some(parse_mode.into());
        self
    }

    pub fn build(self) -> Message {
        Message {
            bot_token: self.bot_token.unwrap_or_default(),
            chat_id: self.chat_id.unwrap_or_default(),
            thread_id: self.thread_id,
            text: self.text.unwrap_or_default(),
            disable_notification: self.disable_notification,
            protect_content: self.protect_content,
            disable_link_preview: self.disable_link_preview,
            parse_mode: self.parse_mode,
        }
    }
}

pub struct TelegramMessageDispatcher {
    error_report_bot_token: Option<String>,
    error_report_chat_id: Option<String>,
    error_report_thread_id: Option<String>,

    client: reqwest::Client,
}

impl TelegramMessageDispatcher {
    pub fn new(
        error_report_bot_token: Option<String>,
        error_report_chat_id: Option<String>,
        error_report_thread_id: Option<String>,
    ) -> Self {
        Self {
            error_report_bot_token,
            error_report_chat_id,
            error_report_thread_id,
            client: reqwest::ClientBuilder::new().build().unwrap(),
        }
    }

    pub fn new_without_error_report() -> Self {
        Self::new(None, None, None)
    }

    fn get_url<T: std::fmt::Display>(bot_token: T) -> String {
        format!("https://api.telegram.org/bot{}/sendMessage", bot_token)
    }

    pub async fn send_message(&self, message: Message) {
        let url = Self::get_url(&message.bot_token);

        let mut data = Map::new();

        data.insert("chat_id".to_string(), json!(&message.chat_id));
        data.insert("text".to_string(), json!(&message.text));
        data.insert(
            "parse_mode".to_string(),
            json!(&message
                .parse_mode
                .clone()
                .unwrap_or("MarkdownV2".to_string())),
        );

        if let Some(thread_id) = &message.thread_id {
            data.insert("message_thread_id".to_string(), json!(thread_id));
        }

        if let Some(disable_notification) = &message.disable_notification {
            data.insert(
                "disable_notification".to_string(),
                json!(disable_notification),
            );
        }

        if let Some(protect_content) = &message.protect_content {
            data.insert("protect_content".to_string(), json!(protect_content));
        }

        if let Some(disable_link_preview) = &message.disable_link_preview {
            data.insert(
                "link_preview_options".to_string(),
                json!({
                    "is_disabled": disable_link_preview,
                }),
            );
        }

        tracing::debug!("sending message to telegram: {data:?}");

        let response = self.client.post(&url).json(&data).send().await;
        if let Err(err) = self.handle_response(response).await {
            tracing::error!("fail to send message to telegram: {err:#}");

            self.report_error(message, format!("{err:#}")).await;
        }
    }

    pub async fn report_error(&self, original_message: Message, error_message: String) {
        let error_report_bot_token = match &self.error_report_bot_token {
            Some(token) => token,
            None => {
                tracing::warn!("telegram message fails to send but error reporting is disabled");
                return;
            }
        };

        let url = Self::get_url(error_report_bot_token);

        let mut data = Map::new();

        data.insert("chat_id".to_string(), json!(self.error_report_chat_id));
        data.insert(
            "link_preview_options".to_string(),
            json!({
                "is_disabled": true,
            }),
        );

        if let Some(thread_id) = &self.error_report_thread_id {
            data.insert("message_thread_id".to_string(), json!(thread_id));
        }

        data.insert(
            "text".to_string(),
            json!(format!(
                "❌ Fail to send message\n\nOriginal message: {}\nError: {error_message}",
                json!(original_message.text)
            )),
        );

        let response = self.client.post(&url).json(&data).send().await;
        if let Err(err) = self.handle_response(response).await {
            tracing::error!("fail to send error report to telegram: {err:#}");
        }
    }

    async fn handle_response(&self, response: reqwest::Result<Response>) -> eyre::Result<()> {
        let response = match response {
            Ok(response) if response.status() != StatusCode::OK => {
                let status = response.status();
                let body = response
                    .text()
                    .await
                    .unwrap_or("fail to read body".to_string());

                eyre::bail!("response status: {status}, body: {body}");
            }
            Ok(response) => response,
            Err(err) => {
                eyre::bail!("failed to send message: {err:#}");
            }
        };

        match response.json::<serde_json::Value>().await {
            Ok(value) => {
                tracing::debug!("response: {value}");
            }
            Err(err) => {
                eyre::bail!("failed to parse response: {err:#}");
            }
        };

        Ok(())
    }
}

impl Default for TelegramMessageDispatcher {
    fn default() -> Self {
        Self::new_without_error_report()
    }
}

#[async_trait]
impl Executor<Message> for TelegramMessageDispatcher {
    fn name(&self) -> &str {
        "TelegramMessageDispatcher"
    }

    async fn execute(&self, action: Message) -> eyre::Result<()> {
        tracing::debug!("received message: {action:?}");

        self.send_message(action).await;

        Ok(())
    }
}

pub fn escape(raw: &str) -> String {
    let escaped_characters = r"\*_[]~`>#-|{}.!+()=";
    let escaped_string: String = raw
        .chars()
        .map(|c| {
            if escaped_characters.contains(c) {
                format!("\\{}", c)
            } else {
                c.to_string()
            }
        })
        .collect();

    escaped_string
}
