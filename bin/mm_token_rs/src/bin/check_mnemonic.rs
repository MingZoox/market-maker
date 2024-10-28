use std::str::FromStr;

use ethers::signers::Signer;
use mm_token_rs::types::CheckMnemonicWalletInfo;
use mm_token_utils::{
    env::get_env,
    log::setup_logger,
    utils::{get_wallet_path_prefix, load_mnemonic_wallet},
};
use provider_utils::enums::ENetwork;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();
    setup_logger(None)?;

    let network_str = get_env("LISTEN_NETWORK", None);
    let Ok(listen_network) = ENetwork::from_str(&network_str) else {
        panic!("LISTEN_NETWORK {:?} invalid", network_str);
    };
    let checked_mnemonic = get_env("CHECKED_MNEMONIC", None);
    let checked_mnemonic_wallet_count: u32 = get_env("CHECKED_MNEMONIC_WALLET_COUNT", None)
        .parse()
        .unwrap();

    let hd_wallet_path_prefix = get_wallet_path_prefix(listen_network);
    let mut list_wallets_info = Vec::<CheckMnemonicWalletInfo>::new();
    for index_wallet in 0..checked_mnemonic_wallet_count {
        let wallet = load_mnemonic_wallet(&checked_mnemonic, index_wallet)?;
        let secret_key = wallet.signer().to_bytes();
        let private_key: String = secret_key
            .iter()
            .map(|b| format!("{:02X}", b).to_lowercase())
            .collect();
        let wallet_info = CheckMnemonicWalletInfo {
            path: hd_wallet_path_prefix.clone() + &index_wallet.to_string(),
            address: wallet.address(),
            private_key,
        };
        list_wallets_info.push(wallet_info);
    }

    log::info!("list_wallets_info: {:#?}", list_wallets_info);

    Ok(())
}
