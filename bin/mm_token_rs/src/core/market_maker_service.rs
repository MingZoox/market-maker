use crate::{
    constants::Env,
    core::{MessageTransportService, WalletService},
    routers::RouterService,
    utils::get_mm_config,
};
use anyhow::anyhow;
use ethers::{
    middleware::SignerMiddleware,
    providers::{Http, Middleware, Provider},
    signers::{LocalWallet, Signer, WalletError},
    types::{Address, U256},
    utils::parse_ether,
};
use mm_token_utils::{
    abi::MemeTokenAbigen, constants::WRAPPED_NATIVE_TOKENS, utils::load_mnemonic_wallet,
};
use provider_utils::{constants::DESERIALIZATION_ERROR_MSG, http_providers::HttpProviders};
use rand::Rng;
use std::{
    sync::{atomic::Ordering, Arc},
    time::Duration,
};
use tokio::{sync::RwLock, task::JoinSet};

use crate::types::{MmConfig, MmSettings};

#[derive(Debug, Clone)]
pub struct MarketMakerService {
    env: Env,
    http_provider: Arc<Provider<Http>>,
    weth_address: Address,
    router_service: RouterService,
}

impl MarketMakerService {
    pub fn new(env: Env, gas_price: Arc<RwLock<U256>>, http_provider: Arc<Provider<Http>>) -> Self {
        let Some(weth) = WRAPPED_NATIVE_TOKENS.get(&env.listen_network) else {
            panic!(
                "WRAPPED_NATIVE_TOKENS not found in {:?}",
                env.listen_network
            );
        };
        Self {
            env: env.clone(),
            http_provider: http_provider.clone(),
            weth_address: weth.address,
            router_service: RouterService::new(env, gas_price, http_provider),
        }
    }

    /// Market make
    /// Increase volume and makers of a token
    pub async fn market_make(&self) -> anyhow::Result<()> {
        let mut set = JoinSet::new();
        let exit = self.env.exit.clone();

        let mm_config: MmConfig = get_mm_config();
        let default_settings = mm_config.default_settings.clone();
        let mm_settings_list: Vec<MmSettings> = mm_config
            .groups
            .clone()
            .iter()
            .map(|settings| MmSettings {
                mnemonic: settings.mnemonic.clone(),
                max_wallets_count: Some(
                    settings
                        .max_wallets_count
                        .unwrap_or(default_settings.max_wallets_count),
                ),
                min_buy_volume: Some(
                    settings
                        .min_buy_volume
                        .unwrap_or(default_settings.min_buy_volume),
                ),
                max_buy_volume: Some(
                    settings
                        .max_buy_volume
                        .unwrap_or(default_settings.max_buy_volume),
                ),
                min_delay_time: Some(
                    settings
                        .min_delay_time
                        .unwrap_or(default_settings.min_delay_time),
                ),
                max_delay_time: Some(
                    settings
                        .max_delay_time
                        .unwrap_or(default_settings.max_delay_time),
                ),
                min_retain_token: Some(
                    settings
                        .min_retain_token
                        .unwrap_or(default_settings.min_retain_token),
                ),
                max_retain_token: Some(
                    settings
                        .max_retain_token
                        .unwrap_or(default_settings.max_retain_token),
                ),
            })
            .collect();

        let provider_index: Arc<RwLock<usize>> = Arc::new(RwLock::new(
            HttpProviders::init_provider_index(&self.env.listen_network, false).await?,
        ));
        let message_transport_service = MessageTransportService::new();
        let message = "Market maker have been launch".to_string();
        message_transport_service.send_message(message).await?;

        set.spawn(HttpProviders::fetch_periodically(
            self.env.listen_network,
            false,
            Some(exit.clone()),
            provider_index.clone(),
        ));
        for (mm_index, mm_settings) in mm_settings_list.iter().enumerate() {
            set.spawn(Self::market_make_by_config(
                self.clone(),
                mm_index,
                mm_settings.to_owned(),
                provider_index.clone(),
            ));
        }

        while let Some(res) = set.join_next().await {
            match res {
                Ok(Ok(())) => {
                    log::info!("Program exited gracefully.");
                }
                Ok(Err(err)) => {
                    log::error!("Error occurred: {:?}", err);
                    exit.store(true, Ordering::Relaxed);
                }
                Err(err) => {
                    log::error!("Error occurred: {:?}", err);
                    exit.store(true, Ordering::Relaxed);
                }
            }
        }

        Ok(())
    }

    async fn market_make_by_config(
        mut self,
        mm_index: usize,
        mm_settings: MmSettings,
        provider_index: Arc<RwLock<usize>>,
    ) -> anyhow::Result<()> {
        log::info!("MM Settings: {:#?}", mm_settings);

        let mm_mnemonic = mm_settings.mnemonic;
        let mm_wallets_size: u32 = mm_settings.max_wallets_count.unwrap();
        let gas_price =
            self.http_provider.get_gas_price().await? * U256::from(101) / U256::from(100);
        let transfer_gas_cost = gas_price * U256::from(21_000);

        // find wallet with enough balance
        let mut index: u32 = 0;
        loop {
            if index >= mm_wallets_size {
                log::error!("cannot find wallet with positive balance, exited");
                break;
            }
            let wallet = self.load_mnemonic_wallet(&mm_mnemonic, index)?;
            let balance = self
                .http_provider
                .get_balance(wallet.address(), None)
                .await?;

            let min_buy_volume = mm_settings.min_buy_volume.unwrap();
            let min_buy_eth_amount = parse_ether(min_buy_volume.to_string()).unwrap();
            if balance > transfer_gas_cost + min_buy_eth_amount {
                break;
            }

            log::info!("wallet {:?} has low eth balance {:?} < transfer_gas_cost + min_buy_eth_amount {:?}, next wallet", wallet.address(), balance, transfer_gas_cost + min_buy_eth_amount);
            index += 1;
        }

        // market make
        let mut is_entire_eth_err = false;
        let message_transport_service = MessageTransportService::new();
        loop {
            // update healthy provider
            self.http_provider = Arc::new(
                HttpProviders::get_provider(
                    &self.env.listen_network,
                    false,
                    provider_index.clone(),
                )
                .await?,
            );

            // check out of bound and refund ETH to first wallet
            if index >= mm_wallets_size {
                log::error!("index outbound, exited");
                let final_wallet = self.load_mnemonic_wallet(&mm_mnemonic, index)?;
                let first_wallet = self.load_mnemonic_wallet(&mm_mnemonic, 0)?;
                log::info!(
                    "start refund the remaining ETH to first wallet: {:#?}",
                    first_wallet.address()
                );

                let final_signer =
                    SignerMiddleware::new(self.http_provider.clone(), final_wallet.clone());

                if let Err(err) = WalletService::send_entire_eth_balance(
                    &final_signer,
                    final_wallet.address(),
                    first_wallet.address(),
                )
                .await
                {
                    log::warn!("rerun because resend overshot failed err={:?}", err);
                    is_entire_eth_err = true;
                    continue;
                } else {
                    let message = format!(
                        "Market maker status \nMarket index: {:#?} \nRefund the remaining ETH to first wallet: {:#?}",
                        mm_index,
                        first_wallet.address(),
                    );
                    message_transport_service.send_message(message).await?;
                    break Ok(());
                }
            }

            let wallet = self.load_mnemonic_wallet(&mm_mnemonic, index)?;
            let next_wallet = self.load_mnemonic_wallet(&mm_mnemonic, index + 1)?;
            let from_address = wallet.address();
            log::info!(
                "market make, from_address {:?}, index {:?}",
                from_address,
                index
            );

            let signer = SignerMiddleware::new(self.http_provider.clone(), wallet.clone());
            if is_entire_eth_err {
                if let Err(err) = WalletService::send_entire_eth_balance(
                    &signer,
                    from_address,
                    next_wallet.address(),
                )
                .await
                {
                    log::warn!("rerun because resend overshot failed err={:?}", err);
                    is_entire_eth_err = true;
                    continue;
                }

                is_entire_eth_err = false;
                index += 1;
                continue;
            }

            let token_contract =
                MemeTokenAbigen::new(self.env.token_address, self.http_provider.clone());

            let min_buy_volume = mm_settings.min_buy_volume.unwrap();
            let max_buy_volume = mm_settings.max_buy_volume.unwrap();

            let num = rand::thread_rng().gen_range(min_buy_volume..=max_buy_volume);
            let eth_amount = parse_ether(num.to_string()).unwrap();
            log::info!("buying token with eth_amount {:?}", num);

            let (pair_address, _) = self
                .router_service
                .get_pair_address(&self.env.token_address, &self.weth_address, true)
                .await?;

            let signed_buy_tx = self
                .router_service
                .construct_buy_token_tx(&wallet, None, eth_amount, &pair_address, true)
                .await?;

            let buy_pending_tx = self
                .http_provider
                .send_raw_transaction(signed_buy_tx)
                .await?;
            let buy_tx_receipt = match buy_pending_tx.await {
                Ok(result) => result,
                Err(err) => {
                    if err.to_string().contains(DESERIALIZATION_ERROR_MSG) {
                        continue;
                    }
                    return Err(err.into());
                }
            };

            let min_delay_time = mm_settings.min_delay_time.unwrap();
            let max_delay_time = mm_settings.max_delay_time.unwrap();
            let sleep_duration =
                Duration::from_secs(rand::thread_rng().gen_range(min_delay_time..=max_delay_time));
            log::info!(
                "token bought tx_hash={:?}, sleep={:?}",
                buy_tx_receipt.map(|x| x.transaction_hash),
                sleep_duration
            );
            tokio::time::sleep(sleep_duration).await;

            let router_address = self.router_service.get_router_address()?;

            let balance_of = token_contract.balance_of(from_address);
            let allowance = token_contract.allowance(from_address, router_address);
            let token_decimals = token_contract.decimals();
            let (token_balance, allowance, token_decimals) =
                tokio::join!(balance_of.call(), allowance.call(), token_decimals.call());
            let token_balance = token_balance?;
            let allowance = allowance?;
            let token_decimals = token_decimals?;

            if allowance < token_balance {
                log::info!("approving token");

                let token_contract =
                    MemeTokenAbigen::new(self.env.token_address, Arc::new(signer.clone()));
                match token_contract
                    .approve(router_address, U256::MAX)
                    .send()
                    .await
                {
                    Ok(result) => result.await?,
                    Err(err) => {
                        if err.to_string().contains(DESERIALIZATION_ERROR_MSG) {
                            continue;
                        }
                        return Err(err.into());
                    }
                };
            }

            log::info!("selling token");
            let min_retain_token = mm_settings.min_retain_token.unwrap();
            let max_retain_token = mm_settings.max_retain_token.unwrap();
            // keep retain token for holder volume
            let retain_token = rand::thread_rng().gen_range(min_retain_token..=max_retain_token);
            log::info!("retain_token: {:#?}", retain_token);
            let retain_token_with_decimals =
                U256::from(retain_token) * U256::exp10(token_decimals as usize);

            if token_balance < retain_token_with_decimals {
                return Err(anyhow!("token_balance must be greater than retain_token"));
            }
            let token_amount_in: U256 = token_balance - retain_token_with_decimals;

            let (pair_address, _) = self
                .router_service
                .get_pair_address(&self.env.token_address, &self.weth_address, false)
                .await?;

            let signed_sell_tx = self
                .router_service
                .construct_sell_token_tx(&wallet, None, token_amount_in, &pair_address, true)
                .await?;

            let sell_pending_tx = self
                .http_provider
                .send_raw_transaction(signed_sell_tx)
                .await?;
            let sell_tx_receipt = match sell_pending_tx.await {
                Ok(result) => result,
                Err(err) => {
                    if err.to_string().contains(DESERIALIZATION_ERROR_MSG) {
                        continue;
                    }
                    return Err(err.into());
                }
            };

            let sleep_duration =
                Duration::from_secs(rand::thread_rng().gen_range(min_delay_time..=max_delay_time));
            log::info!(
                "token sold tx_hash={:?}, sleep={:?}",
                sell_tx_receipt.map(|x| x.transaction_hash),
                sleep_duration
            );
            tokio::time::sleep(sleep_duration).await;

            log::info!(
                "migrate to next_wallet={:?}, next_index={:?}",
                next_wallet.address(),
                index + 1
            );
            if let Err(err) =
                WalletService::send_entire_eth_balance(&signer, from_address, next_wallet.address())
                    .await
            {
                log::warn!("rerun because resend overshot failed err={:?}", err);
                is_entire_eth_err = true;
                continue;
            }

            let message = format!(
                "Market maker status \nMarket index: {:#?} \nMigrate to next_wallet={:?}, next_index={:?}",
                mm_index,
                next_wallet.address(),
                index + 1
            );
            message_transport_service.send_message(message).await?;

            index += 1;
        }
    }

    pub fn load_mnemonic_wallet(
        &self,
        mnemonic: &str,
        index: u32,
    ) -> Result<LocalWallet, WalletError> {
        let wallet = load_mnemonic_wallet(mnemonic, index)?;
        let wallet = wallet.with_chain_id(self.env.chain_id.as_u64());
        Ok(wallet)
    }
}
