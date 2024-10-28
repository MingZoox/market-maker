use std::{collections::HashMap, fs::File, io::Read, path::Path, str::FromStr, sync::Arc};

use ethers::{
    providers::{Http, Middleware, Provider},
    signers::Signer,
    types::{Address, U256},
    utils::parse_ether,
};
use mm_token_utils::{abi::MemeTokenAbigen, utils::load_mnemonic_wallet};
use provider_utils::enums::ENetwork;
use rust_decimal::Decimal;
use tokio::sync::RwLock;

use crate::types::MmConfig;

/**
 * get all system wallet nonces and balances
 */
pub async fn compute_system_wallets(
    mnemonic: &str,
    wallets_size: u32,
    token_address: &Address,
    http_provider: Arc<Provider<Http>>,
) -> anyhow::Result<HashMap<Address, Arc<RwLock<WalletContext>>>> {
    let mut addresses = HashMap::new();
    let token_contract = MemeTokenAbigen::new(*token_address, http_provider.clone());

    for index in 0..wallets_size {
        let wallet = load_mnemonic_wallet(mnemonic, index)?;
        let wallet_address = wallet.address();
        let balance_of = token_contract.balance_of(wallet_address);
        let (token_balance, eth_balance, nonce) = tokio::join!(
            balance_of.call(),
            http_provider.get_balance(wallet_address, None),
            http_provider.get_transaction_count(wallet_address, None)
        );
        let token_balance = token_balance?;
        let eth_balance = eth_balance?;
        let nonce = nonce?;

        addresses.insert(
            wallet_address,
            Arc::new(RwLock::new(WalletContext {
                index,
                address: wallet_address,
                nonce,
                token_balance,
                eth_balance,
            })),
        );
    }

    Ok(addresses)
}

pub fn load_system_wallet_address(
    mnemonic: &str,
    wallets_size: u32,
) -> anyhow::Result<Vec<Address>> {
    let mut addresses = Vec::new();
    for index in 0..wallets_size {
        let wallet = load_mnemonic_wallet(mnemonic, index)?;
        let wallet_address = wallet.address();
        addresses.push(wallet_address)
    }

    Ok(addresses)
}

pub async fn compute_all_system_wallets(
    auto_buyer_mnemonic: &str,
    auto_buyer_wallets_count: u32,
    buyer_mnemonic: &str,
    buyer_wallets_count: u32,
    seller_mnemonic: &str,
    seller_wallets_count: u32,
) -> anyhow::Result<(Vec<Address>, Vec<Address>, Vec<Address>, Vec<Address>)> {
    let auto_buyer_system_wallets =
        load_system_wallet_address(auto_buyer_mnemonic, auto_buyer_wallets_count)?;
    let buyer_system_wallets = load_system_wallet_address(buyer_mnemonic, buyer_wallets_count)?;
    let seller_system_wallets = load_system_wallet_address(seller_mnemonic, seller_wallets_count)?;

    let mm_config = get_mm_config();
    let default_settings = mm_config.default_settings.clone();
    // get mnemonic and number of wallet market maker
    let mm_wallet_settings_list: Vec<(String, u32)> = mm_config
        .groups
        .clone()
        .iter()
        .map(|settings| {
            (
                settings.mnemonic.clone(),
                settings
                    .max_wallets_count
                    .unwrap_or(default_settings.max_wallets_count),
            )
        })
        .collect();

    let mut market_maker_system_wallets = Vec::new();
    for (mm_mnemonic, wallet_count) in mm_wallet_settings_list {
        let mm_mnemonic_wallets = load_system_wallet_address(&mm_mnemonic, wallet_count)?;
        market_maker_system_wallets.extend(mm_mnemonic_wallets);
    }

    Ok((
        auto_buyer_system_wallets,
        buyer_system_wallets,
        seller_system_wallets,
        market_maker_system_wallets,
    ))
}

pub fn format_bmk(number: &str, dp: u32) -> Result<String, rust_decimal::Error> {
    let decimal_number = Decimal::from_str(number)?;
    if decimal_number >= Decimal::from(1_000_000_000) {
        let x = (decimal_number / Decimal::from(1_000_000_000)).round_dp(dp);
        return Ok(format!("{}B", x));
    }
    if decimal_number >= Decimal::from(1_000_000) {
        let x = (decimal_number / Decimal::from(1_000_000)).round_dp(dp);
        return Ok(format!("{}M", x));
    }
    if decimal_number >= Decimal::from(1_000) {
        let x = (decimal_number / Decimal::from(1_000)).round_dp(dp);
        return Ok(format!("{}K", x));
    }

    Ok(decimal_number.round_dp(dp).to_string())
}

pub fn read_json_file(file_path: &str) -> std::io::Result<String> {
    let path = Path::new(file_path);
    let mut file = File::open(path)?;
    let mut content = String::new();
    file.read_to_string(&mut content)?;
    Ok(content)
}

pub fn get_mm_config() -> MmConfig {
    let file_path = "mm_config.json";
    let json_content = read_json_file(file_path).expect("Failed to read JSON file");
    let mm_config: MmConfig = serde_json::from_str(&json_content).expect("Failed to parse JSON");
    mm_config
}

#[derive(Debug, Default, Clone)]
pub struct WalletContext {
    pub index: u32,
    pub address: Address,
    pub nonce: U256,
    pub token_balance: U256,
    pub eth_balance: U256,
}

pub fn get_bloxroute_tip_fee(network: &ENetwork, number_of_txs: u32) -> U256 {
    if ![ENetwork::BscMainnet, ENetwork::BscTestnet].contains(network) {
        return U256::zero();
    }

    match number_of_txs {
        0..=2 => parse_ether("0.0004").unwrap(),
        3..=5 => parse_ether("0.004").unwrap(),
        6..=10 => parse_ether("0.008").unwrap(),
        11..=15 => parse_ether("0.012").unwrap(),
        _ => parse_ether("0.012").unwrap(),
    }
}
