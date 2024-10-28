use std::{env, sync::Arc};

use ethers::utils::parse_ether;
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
    if args.len() != 4 {
        log::warn!(
            "Function {} need 3 params: <DISPERSE_ETH_AMOUNT> <DISPERSE_ETH_WALLET_INDEX_FROM> <DISPERSE_ETH_WALLET_INDEX_TO>",
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
    let disperse_eth_private_key = get_env("DISPERSE_ETH_PRIVATE_KEY", None);
    let disperse_eth_mnemonic = get_env("DISPERSE_ETH_MNEMONIC", None);

    let disperse_eth_amount = parse_ether(args[1].parse::<String>().unwrap())?;
    let disperse_eth_wallet_index_from: u32 = args[2].parse().unwrap();
    let disperse_eth_wallet_index_to: u32 = args[3].parse().unwrap();
    if disperse_eth_wallet_index_from > disperse_eth_wallet_index_to {
        log::warn!("Please set DISPERSE_ETH_WALLET_INDEX_FROM lower or equal than DISPERSE_ETH_WALLET_INDEX_TO!");
        return Ok(());
    }

    log::info!(
        "Config params:\nDISPERSE_ETH_AMOUNT: {:#?}\nDISPERSE_ETH_WALLET_INDEX_FROM: {:#?}\nDISPERSE_ETH_WALLET_INDEX_TO: {:#?}",
        args[1].parse::<f64>().unwrap(),
        disperse_eth_wallet_index_from,
        disperse_eth_wallet_index_to
    );

    wallet_service
        .disperse_eth(
            &disperse_eth_private_key,
            &disperse_eth_mnemonic,
            disperse_eth_amount,
            disperse_router,
            disperse_eth_wallet_index_from,
            disperse_eth_wallet_index_to,
        )
        .await?;
    Ok(())
}
