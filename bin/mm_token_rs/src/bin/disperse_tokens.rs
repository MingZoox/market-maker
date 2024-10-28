use std::{env, sync::Arc};

use mm_token_rs::{constants::Env, core::WalletService};
use mm_token_utils::{
    constants::{DISPERSE_ROUTERS, ZERO_ADDRESS},
    env::get_env,
    log::setup_logger,
};
use provider_utils::http_providers::HttpProviders;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();
    setup_logger(None)?;

    let args: Vec<String> = env::args().collect();
    if args.len() != 5 {
        log::warn!(
            "Function {} need 4 params: <DISPERSE_TOKEN_WALLET_INDEX_FROM> <DISPERSE_TOKEN_WALLET_INDEX_TO> <DISPERSE_TOKEN_AMOUNT_MIN> <DISPERSE_TOKEN_AMOUNT_MAX>",
            args[0]
        );
        return Ok(());
    }

    let env = Env::new();
    let http_provider =
        Arc::new(HttpProviders::get_healthy_provider(&env.listen_network, false).await?);
    let wallet_service = WalletService::new(env.clone(), http_provider);
    let disperse_router = *DISPERSE_ROUTERS.get(&env.listen_network).unwrap();
    if disperse_router == *ZERO_ADDRESS {
        log::warn!(
            "Please config disperse router for {:#?} network",
            env.listen_network
        );
        return Ok(());
    }

    let disperse_token_private_key = get_env("DISPERSE_TOKEN_PRIVATE_KEY", None);
    let disperse_token_mnemonic = get_env("DISPERSE_TOKEN_MNEMONIC", None);

    let wallet_index_from: u32 = args[1].parse().unwrap();
    let wallet_index_to: u32 = args[2].parse().unwrap();
    let disperse_token_amount_min: u128 = args[3].parse().unwrap();
    let disperse_token_amount_max: u128 = args[4].parse().unwrap();

    if wallet_index_from > wallet_index_to {
        log::warn!("Please set DISPERSE_TOKEN_WALLET_INDEX_FROM lower or equal than DISPERSE_TOKEN_WALLET_INDEX_TO!");
        return Ok(());
    }

    if disperse_token_amount_min > disperse_token_amount_max {
        log::warn!("Please set TOKEN_AMOUNT_MIN lower or equal than TOKEN_AMOUNT_MAX!");
        return Ok(());
    }

    log::info!(
        "Config params:\nDISPERSE_TOKEN_WALLET_INDEX_FROM: {:#?}\nDISPERSE_TOKEN_WALLET_INDEX_TO: {:#?}\nDISPERSE_TOKEN_AMOUNT_MIN: {:#?}\nDISPERSE_TOKEN_AMOUNT_MIN: {:#?}",
        wallet_index_from,
        wallet_index_to,
        disperse_token_amount_min,
        disperse_token_amount_max
    );

    wallet_service
        .disperse_tokens(
            disperse_router,
            &disperse_token_private_key,
            &disperse_token_mnemonic,
            wallet_index_from,
            wallet_index_to,
            disperse_token_amount_min,
            disperse_token_amount_max,
        )
        .await?;

    Ok(())
}
