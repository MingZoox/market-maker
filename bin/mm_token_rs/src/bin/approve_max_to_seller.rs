use std::{env, sync::Arc};

use mm_token_rs::{constants::Env, core::WalletService};
use mm_token_utils::{constants::UNISWAP2_ROUTERS, log::setup_logger};
use provider_utils::http_providers::HttpProviders;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();
    setup_logger(None)?;

    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        log::warn!(
            "Function {} need 2 params: <APPROVE_SELLER_WALLET_INDEX_FROM> <APPROVE_SELLER_WALLET_INDEX_TO>",
            args[0]
        );
        return Ok(());
    }

    let approve_seller_wallet_index_from: u32 = args[1].parse().unwrap();
    let approve_seller_wallet_index_to: u32 = args[2].parse().unwrap();

    if approve_seller_wallet_index_from > approve_seller_wallet_index_to {
        log::warn!("Please set APPROVE_SELLER_WALLET_INDEX_FROM lower or equal than APPROVE_SELLER_WALLET_INDEX_TO!");
        return Ok(());
    }

    let env = Env::new();
    let Some(uniswapv2_router_address) = UNISWAP2_ROUTERS.get(&env.listen_network) else {
        panic!("UNISWAP2_ROUTERS not found in {:?}", env.listen_network);
    };
    let http_provider =
        Arc::new(HttpProviders::get_healthy_provider(&env.listen_network, false).await?);

    let wallet_service = WalletService::new(env, http_provider);

    log::info!(
        "Config params:\nAPPROVE_SELLER_WALLET_INDEX_FROM: {:#?}\nAPPROVE_SELLER_WALLET_INDEX_TO: {:#?}",
        approve_seller_wallet_index_from,
        approve_seller_wallet_index_to
    );

    wallet_service
        .approve_max_to_seller(
            uniswapv2_router_address,
            approve_seller_wallet_index_from,
            approve_seller_wallet_index_to,
        )
        .await?;
    Ok(())
}
