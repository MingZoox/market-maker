use ethers::{
    middleware::SignerMiddleware,
    signers::{LocalWallet, Signer},
    types::H160,
};
use mm_token_rs::{constants::Env, core::WalletService};
use mm_token_utils::{abi::MemeTokenControllerAbigen, env::get_env, log::setup_logger};
use provider_utils::http_providers::HttpProviders;
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();
    setup_logger(None)?;

    let env = Env::new();
    let http_provider =
        Arc::new(HttpProviders::get_healthy_provider(&env.listen_network, false).await?);
    // let token_contract = MemeTokenAbigen::new(env.token_address, http_provider.clone());

    let deployer_private_key = get_env("DEPLOYER_PRIVATE_KEY", Some("".to_string()));
    let deployer_wallet = deployer_private_key
        .parse::<LocalWallet>()
        .unwrap()
        .with_chain_id(env.clone().chain_id.as_u64());

    let wallet_service = WalletService::new(env.clone(), http_provider.clone());
    let buyer_wallets_count: u32 = get_env("BUYER_WALLETS_COUNT", None).parse().unwrap();

    let mut whitelist = Vec::new();
    for wallet_index in 0..buyer_wallets_count {
        let buyer_wallet = wallet_service.load_buyer_wallets(wallet_index)?;
        whitelist.push(buyer_wallet.address());
    }

    let signer = Arc::new(SignerMiddleware::new(
        http_provider.clone(),
        deployer_wallet,
    ));

    let meme_token_controller_address: H160 = get_env("MM_TOKEN_CONTROLLER_ADDRESS", None)
        .parse()
        .unwrap();
    let meme_token_controller =
        MemeTokenControllerAbigen::new(meme_token_controller_address, signer);

    let set_whitelist_fn =
        meme_token_controller.set_multi_whitelist_abcxyz(env.token_address, whitelist, true);
    let tx = set_whitelist_fn.send().await?;

    log::info!("Set whitelist wallets at tx {:#?}", tx.tx_hash());

    Ok(())
}
