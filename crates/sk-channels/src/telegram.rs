use crate::types::{ChannelAdapter, ChannelContent, ChannelMessage, ChannelType, ChannelUser};
use async_trait::async_trait;
use chrono::Utc;
use futures::Stream;
use std::pin::Pin;
use teloxide::prelude::*;
use teloxide::types::ChatId;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tracing::{error, info, warn};

/// Adapter for Telegram built on `teloxide`.
pub struct TelegramAdapter {
    bot: Bot,
    #[allow(dead_code)]
    bot_name: String,
}

impl TelegramAdapter {
    pub async fn new(token: String) -> Result<Self, Box<dyn std::error::Error>> {
        let bot = Bot::new(token);
        let me = bot.get_me().await?;
        let bot_name = me
            .user
            .username
            .clone()
            .unwrap_or_else(|| "unknown_bot".to_string());
        info!("Initialized Telegram adapter for @{}", bot_name);
        Ok(Self { bot, bot_name })
    }
}

#[async_trait]
impl ChannelAdapter for TelegramAdapter {
    fn name(&self) -> &str {
        "telegram"
    }

    fn channel_type(&self) -> ChannelType {
        ChannelType::Telegram
    }

    async fn start(
        &self,
    ) -> Result<Pin<Box<dyn Stream<Item = ChannelMessage> + Send>>, Box<dyn std::error::Error>>
    {
        let (tx, rx) = mpsc::channel(100);
        let bot = self.bot.clone();

        info!("Starting Telegram message listener...");

        tokio::spawn(async move {
            teloxide::repl(bot, move |_bot: Bot, msg: Message| {
                let tx = tx.clone();
                async move {
                    if let Some(text) = msg.text() {
                        let sender_name = msg
                            .from
                            .as_ref()
                            .and_then(|u| u.username.clone())
                            .unwrap_or_else(|| "Unknown".to_string());
                        let is_group = msg.chat.is_group() || msg.chat.is_supergroup();
                        let thread_id = msg.thread_id.map(|id| id.to_string());

                        let channel_msg = ChannelMessage {
                            channel: ChannelType::Telegram,
                            platform_message_id: msg.id.0.to_string(),
                            sender: ChannelUser {
                                platform_id: msg.chat.id.0.to_string(), // Route replies back to this chat
                                display_name: sender_name,
                                sk_user: None,
                            },
                            content: ChannelContent::Text(text.to_string()),
                            target_agent: None,
                            timestamp: Utc::now(),
                            is_group,
                            thread_id,
                            metadata: std::collections::HashMap::new(),
                        };

                        if let Err(e) = tx.send(channel_msg).await {
                            error!("Failed to route Telegram message to kernel: {}", e);
                        }
                    }
                    Ok(())
                }
            })
            .await;
        });

        Ok(Box::pin(ReceiverStream::new(rx)))
    }

    async fn send(
        &self,
        user: &ChannelUser,
        content: ChannelContent,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let chat_id = ChatId(user.platform_id.parse::<i64>().unwrap_or(0));

        match content {
            ChannelContent::Text(text) => {
                // Formatting is handled by router now
                self.bot.send_message(chat_id, text).await?;
            }
            _ => {
                warn!("Unsupported content type for Telegram: {:?}", content);
            }
        }
        Ok(())
    }

    async fn send_typing(&self, user: &ChannelUser) -> Result<(), Box<dyn std::error::Error>> {
        let chat_id = ChatId(user.platform_id.parse::<i64>().unwrap_or(0));
        self.bot
            .send_chat_action(chat_id, teloxide::types::ChatAction::Typing)
            .await?;
        Ok(())
    }

    async fn stop(&self) -> Result<(), Box<dyn std::error::Error>> {
        info!("Stopping Telegram adapter");
        Ok(())
    }
}
