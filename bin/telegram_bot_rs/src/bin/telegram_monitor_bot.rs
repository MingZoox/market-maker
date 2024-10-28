use mm_token_utils::{env::get_env, log::setup_logger};
use telegram_bot_rs::{core::CommandService, types::BotCommand};
use teloxide::prelude::*;
use teloxide::utils::command::BotCommands;

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    let _ = setup_logger(None);
    let telegram_bot_token = get_env("TELEGRAM_BOT_TOKEN", None);
    log::info!("Starting monitor bot...");
    let telegram_bot = Bot::new(telegram_bot_token);

    BotCommand::repl(telegram_bot.clone(), answer).await;
}

async fn answer(bot: Bot, msg: Message, cmd: BotCommand) -> ResponseResult<()> {
    // let env = Env::new();
    let command_service = CommandService::new();

    match cmd {
        BotCommand::Help => {
            bot.send_message(msg.chat.id, BotCommand::descriptions().to_string())
                .await?
        }
        // call APIs command
        BotCommand::GetNetworkStatus => {
            let response = command_service.get_network_status().await;
            bot.send_message(msg.chat.id, format!("Response: {:#?}.", response))
                .await?
        }
        // BotCommand::GetDeploymentChecklist => {
        //     let response = command_service.get_deployment_checklist().await;
        //     bot.send_message(msg.chat.id, format!("Response: {:#?}.", response))
        //         .await?
        // }
        BotCommand::GetDeployer => {
            let response = command_service.get_deployer().await;
            bot.send_message(msg.chat.id, format!("Response: {:#?}.", response))
                .await?
        }
        BotCommand::GetBuyers => {
            let response = command_service.get_buyers().await;
            bot.send_message(msg.chat.id, format!("Response: {:#?}.", response))
                .await?
        }
        BotCommand::GetAutoBuyers => {
            let response = command_service.get_auto_buyers().await;
            bot.send_message(msg.chat.id, response).await?
        }
        BotCommand::GetSellers => {
            let response = command_service.get_sellers().await;
            bot.send_message(msg.chat.id, response).await?
        }
        BotCommand::GetMarketMakers => {
            let response = command_service.get_market_makers().await;
            bot.send_message(msg.chat.id, format!("Response: {:#?}.", response))
                .await?
        }
        BotCommand::LaunchProcess => {
            let response = command_service.launch_process().await;
            bot.send_message(msg.chat.id, format!("Response: {:#?}.", response))
                .await?
        } // launch process command
          // BotCommand::LaunchBuyBot => {
          //     let response = command_service.launch_buy_bot().await;
          //     bot.send_message(msg.chat.id, format!("Response: {:#?}.", response))
          //         .await?
          // }
          // BotCommand::LaunchSellBot => {
          //     let response = command_service.launch_sell_bot().await;
          //     bot.send_message(msg.chat.id, format!("Response: {:#?}.", response))
          //         .await?
          // }
    };

    Ok(())
}
