use std::{str::FromStr, sync::Arc};

use ethers::{
    abi::Address,
    middleware::SignerMiddleware,
    providers::{Http, Provider},
    signers::{LocalWallet, Signer, WalletError},
    types::{TransactionReceipt, U256},
};
use mm_token_utils::{abi::IUniswapV2PairAbigen, env::get_env, utils::load_mnemonic_wallet};
use provider_utils::http_providers::HttpProviders;

use crate::{constants::Env, core::WalletService};

pub struct MigrationService {
    env: Env,
    migration_source_mnemonic: String,
    migration_wallets_count: u32,
    migration_destination_wallet: Address,
    http_provider: Arc<Provider<Http>>,
}

impl MigrationService {
    pub fn new() -> Self {
        let env = Env::new();
        let Ok(http_provider) = HttpProviders::get_first_provider(&env.listen_network, false)
        else {
            panic!("http_provider not found");
        };

        Self {
            env,
            migration_source_mnemonic: get_env("MIGRATION_SOURCE_MNEMONIC", None),
            migration_wallets_count: get_env("MIGRATION_WALLETS_COUNT", Some("0".to_string()))
                .parse()
                .unwrap(),
            migration_destination_wallet: Address::from_str(&get_env(
                "MIGRATION_DESTINATION_WALLET",
                None,
            ))
            .unwrap(),
            http_provider: Arc::new(http_provider),
        }
    }

    /// Migrate all wallets' token to another wallets
    pub async fn migrate_all_token(&self) -> anyhow::Result<()> {
        let mut index = 0;
        while index < self.migration_wallets_count {
            let wallet = self.load_migration_wallet(index)?;
            let (from_wallet_address, to_wallet_address) =
                (wallet.address(), self.migration_destination_wallet);
            log::info!(
                "migrate token index {:?} from_wallet {:?} to_wallet {:?} processing",
                index,
                from_wallet_address,
                to_wallet_address
            );

            let signer = SignerMiddleware::new(self.http_provider.clone(), wallet);
            let token = IUniswapV2PairAbigen::new(self.env.token_address, Arc::new(signer.clone()));
            let token_balance: U256 = token.balance_of(from_wallet_address).call().await?;
            if token_balance > U256::zero() {
                let tx_receipt: Option<TransactionReceipt> = token
                    .transfer(to_wallet_address, token_balance)
                    .send()
                    .await?
                    .await?;
                log::info!(
                    "sent token tx_hash={:?}",
                    tx_receipt.map(|x| x.transaction_hash)
                );
            } else {
                log::warn!("skip because of zero token balance");
            }

            index += 1;
        }

        Ok(())
    }

    /// Migrate all wallets' eth to another wallets
    pub async fn migrate_all_eth(&self) -> anyhow::Result<()> {
        let mut index = 0;
        while index < self.migration_wallets_count {
            let wallet = self.load_migration_wallet(index)?;
            let (from_wallet_address, to_wallet_address) =
                (wallet.address(), self.migration_destination_wallet);
            log::info!(
                "migrate eth index {:?} from_wallet {:?} to_wallet {:?} processing",
                index,
                from_wallet_address,
                to_wallet_address
            );

            let signer = SignerMiddleware::new(self.http_provider.clone(), wallet);

            if let Err(err) = WalletService::send_entire_eth_balance(
                &signer,
                from_wallet_address,
                to_wallet_address,
            )
            .await
            {
                log::warn!("rerun because resend overshot failed err={:?}", err);
                continue;
            }

            index += 1;
        }

        Ok(())
    }

    fn load_migration_wallet(&self, index: u32) -> Result<LocalWallet, WalletError> {
        let wallet = load_mnemonic_wallet(&self.migration_source_mnemonic, index)?;
        Ok(wallet.with_chain_id(self.env.chain_id.as_u64()))
    }
}

impl Default for MigrationService {
    fn default() -> Self {
        Self::new()
    }
}
