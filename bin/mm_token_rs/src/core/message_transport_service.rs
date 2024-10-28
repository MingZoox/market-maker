use mm_token_utils::env::get_env;
use teloxide::prelude::*;

use crate::types::TelegramConfig;

#[derive(Debug, Clone)]
pub struct MessageTransportService {
    telegram_enabled: bool,
    telegram_config: Option<TelegramConfig>,
    telegram_bot: Option<Bot>,
    // email_enabled: bool,
    // email_sender_address: Option<String>,
    // email_sender_password: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransportPlatform {
    TELEGRAM,
    EMAIL,
    SLACK,
    DISCORD,
}

impl Default for MessageTransportService {
    fn default() -> Self {
        Self::new()
    }
}

impl MessageTransportService {
    pub fn new() -> Self {
        let telegram_enabled: bool = get_env("TELEGRAM_ENABLED", None).parse().unwrap();
        let telegram_bot_token = get_env("TELEGRAM_BOT_TOKEN", None);
        let telegram_channel_id = get_env("TELEGRAM_CHANNEL_ID", None);

        // let email_enabled: bool = get_env("EMAIL_ENABLED", None).parse().unwrap();
        Self {
            telegram_enabled,
            telegram_config: if telegram_enabled {
                Some(TelegramConfig {
                    telegram_bot_token: telegram_bot_token.clone(),
                    telegram_channel_id,
                })
            } else {
                None
            },
            telegram_bot: if telegram_enabled {
                Some(Bot::new(telegram_bot_token))
            } else {
                None
            },
            // email_enabled,
            // email_sender_address: if email_enabled {
            //     Some(get_env("EMAIL_SENDER_ADDRESS", None))
            // } else {
            //     None
            // },
            // email_sender_password: if email_enabled {
            //     Some(get_env("EMAIL_SENDER_PASSWORD", None))
            // } else {
            //     None
            // },
        }
    }

    pub async fn send_message(&self, message: String) -> anyhow::Result<()> {
        if self.telegram_enabled {
            Self::handle_send_telegram(self, self.telegram_bot.clone().unwrap(), message.clone())
                .await?;
        }

        // if self.email_enabled {
        //     Self::handle_send_email(self, message.clone()).await?;
        // }
        Ok(())
        // TransportPlatform::DISCORD => Ok(()),
        // TransportPlatform::SLACK => Ok(()),
    }

    async fn handle_send_telegram(&self, telegram_bot: Bot, message: String) -> anyhow::Result<()> {
        log::info!("Sending message to telegram bot...");
        telegram_bot
            .send_message(
                self.telegram_config.clone().unwrap().telegram_channel_id,
                message,
            )
            .await
            .unwrap();

        Ok(())
    }

    // async fn handle_send_email(&self, message: String) -> anyhow::Result<()> {
    //     log::info!("Sending message to list emails...");

    //     let email = Message::builder()
    //         .from(self.email_sender_address.clone().unwrap().parse().unwrap())
    //         .to("receipt email".parse().unwrap())
    //         .subject("Monitor Token Launch")
    //         .header(ContentType::TEXT_PLAIN)
    //         .body(message)
    //         .unwrap();

    //     let creds = Credentials::new("smtp_username".to_owned(), "smtp_password".to_owned());

    //     // Open a remote connection to gmail
    //     let mailer = SmtpTransport::relay("smtp.gmail.com")
    //         .unwrap()
    //         .credentials(creds)
    //         .build();

    //     // Send the email
    //     match mailer.send(&email) {
    //         Ok(_) => log::info!("Email sent successfully!"),
    //         Err(e) => panic!("Could not send email: {e:?}"),
    //     }
    //     Ok(())
    // }
}
