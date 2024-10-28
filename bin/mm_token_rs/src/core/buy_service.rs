use crate::{
    constants::Env,
    core::MessageTransportService,
    routers::RouterService,
    types::TokenInfo,
    utils::{compute_system_wallets, WalletContext},
};
use anyhow::anyhow;
use ethers::{
    providers::{Http, Middleware, Provider},
    signers::{LocalWallet, Signer, WalletError},
    types::{Address, Bytes, U256, U64},
    utils::{format_ether, parse_ether},
};
use futures::{future::join_all, FutureExt};
use mm_token_utils::{
    abi::MemeTokenAbigen,
    constants::WRAPPED_NATIVE_TOKENS,
    env::get_env,
    utils::{compute_transaction_hash, load_mnemonic_wallet},
};
use provider_utils::{constants::DESERIALIZATION_ERROR_MSG, http_providers::HttpProviders};
use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio::{sync::RwLock, task, time::timeout};

#[derive(Debug, Clone)]
pub struct BuyService {
    env: Env,
    http_provider: Arc<Provider<Http>>,
    weth_address: Address,
    token_info: TokenInfo,
    provider_index: Arc<RwLock<usize>>,
    buyer_mnemonic: String,
    buyer_surplus_balance: U256,
    buyer_wallets_count: u32,
    router_service: RouterService,
}

impl BuyService {
    pub fn new(
        env: Env,
        gas_price: Arc<RwLock<U256>>,
        provider_index: Arc<RwLock<usize>>,
        http_provider: Arc<Provider<Http>>,
    ) -> Self {
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
            token_info: TokenInfo::default(),
            provider_index,
            buyer_mnemonic: get_env("BUYER_MNEMONIC", None),
            buyer_surplus_balance: parse_ether(get_env("BUYER_SURPLUS_BALANCE", None)).unwrap(),
            buyer_wallets_count: get_env("BUYER_WALLETS_COUNT", None).parse().unwrap(),
            router_service: RouterService::new(env, gas_price, http_provider),
        }
    }

    pub async fn init(&mut self) -> anyhow::Result<()> {
        let token_info_call =
            MemeTokenAbigen::new(self.env.token_address, self.http_provider.clone());
        let symbol: String = token_info_call.symbol().call().await.unwrap();
        let name: String = token_info_call.name().call().await.unwrap();
        let decimals: u8 = token_info_call.decimals().call().await.unwrap();
        let total_supply: U256 = token_info_call.total_supply().call().await.unwrap();

        self.token_info = TokenInfo {
            address: self.env.token_address,
            symbol,
            name,
            decimals,
            total_supply,
        };

        Ok(())
    }

    pub async fn start_event_mode(&self) -> anyhow::Result<()> {
        let message_transport_service = MessageTransportService::new();
        let message = "Buy service have been launch".to_string();
        message_transport_service.send_message(message).await?;

        let system_wallets = compute_system_wallets(
            &self.buyer_mnemonic,
            self.buyer_wallets_count,
            &self.env.token_address,
            self.http_provider.clone(),
        )
        .await?;

        let mut wallet_configs: Vec<(usize, Address)> = Vec::new(); // (wallet_index, wallet_address)

        for (wallet_index, (wallet_address, wallet)) in system_wallets.iter().enumerate() {
            // try write, if wallet is in use, skip it
            let _wallet_lock = match wallet.try_write() {
                Ok(wallet_lock) => wallet_lock,
                Err(_) => continue,
            };

            wallet_configs.push((wallet_index, *wallet_address));
        }

        self.listen(&system_wallets, wallet_configs).await?;
        Ok(())
    }

    async fn listen(
        &self,
        system_wallets: &HashMap<Address, Arc<RwLock<WalletContext>>>,
        wallet_configs: Vec<(usize, Address)>,
    ) -> anyhow::Result<()> {
        let mut futures = Vec::new();
        for (_, wallet_address) in wallet_configs {
            let Some(wallet_context) = system_wallets.get(&wallet_address) else {
                continue;
            };
            let wallet_context = wallet_context.clone();
            let mut buy_service = self.clone();

            futures.push(
                task::spawn(async move {
                    let _ = buy_service.handle_buy_one(wallet_context.clone()).await;
                })
                .boxed(),
            );
        }
        join_all(futures).await;

        Ok(())
    }

    async fn handle_buy_one(
        &mut self,
        wallet_context: Arc<RwLock<WalletContext>>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        loop {
            // get healthy provider
            self.http_provider = Arc::new(
                HttpProviders::get_provider(
                    &self.env.listen_network,
                    false,
                    self.provider_index.clone(),
                )
                .await?,
            );

            let should_next = match self.try_buy(&wallet_context).await {
                Ok(should_next) => should_next,
                Err(err) => {
                    if err.to_string().contains(DESERIALIZATION_ERROR_MSG) {
                        continue;
                    }
                    return Err(err);
                }
            };

            if !should_next {
                tokio::time::sleep(Duration::from_secs(2)).await;
                break;
            }

            tokio::time::sleep(Duration::from_secs(1)).await;
        }
        Ok(())
    }

    async fn try_buy(
        &self,
        wallet_context: &Arc<RwLock<WalletContext>>,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        let message_transport_service = MessageTransportService::new();
        let mut wallet_context_mut = wallet_context.write().await;

        let wallet = self.load_wallet(wallet_context_mut.index)?;

        if wallet_context_mut.eth_balance <= self.buyer_surplus_balance {
            println!(
                "[BuyService] Wallet [{:?}] balance is less than threshold.",
                wallet_context_mut.address,
            );
            return Ok(false);
        }
        let buy_amount = wallet_context_mut.eth_balance - self.buyer_surplus_balance;

        let (pair_address, _) = match self
            .router_service
            .get_pair_address(&self.env.token_address, &self.weth_address, true)
            .await
        {
            Ok(pair_address) => pair_address,
            Err(err) => {
                println!("[BuyService] Error getting pair address: {:?}", err);
                return Ok(true);
            }
        };

        println!(
            "[BuyService] Trying to buy:
                - Wallet Index: {:?} - Wallet Address: {:?}
                - Token: {:?} - Amount: {:?} - Pair Address: {:?}",
            wallet_context_mut.index,
            wallet.address(),
            self.env.token_address,
            buy_amount,
            pair_address
        );

        let token_price = self
            .router_service
            .get_token_native_price(self.router_service.active_router, pair_address)
            .await?;

        let signed_buy_tx = match self
            .router_service
            .construct_buy_token_tx(
                &wallet,
                Some(wallet_context_mut.nonce),
                buy_amount,
                &pair_address,
                true,
            )
            .await
        {
            Ok(signed_buy_tx) => signed_buy_tx,
            Err(err) => {
                println!("[BuyService] try_buy {:?}", err);
                return Ok(true);
            }
        };

        let buy_tx_hash = compute_transaction_hash(&signed_buy_tx);

        let pending_tx = self.http_provider.send_raw_transaction(signed_buy_tx).await;

        match pending_tx {
            Ok(pending_tx) => {
                let tx_receipt = timeout(Duration::from_secs(3), pending_tx)
                    .await
                    .map_err(|err| anyhow!("Timeout occurred: {}", err))??
                    .ok_or_else(|| anyhow!("Cannot find tx_receipt"))?;

                let message: String = if tx_receipt.status == Some(U64::zero()) {
                    println!("Buy transaction {:#?} failed", buy_tx_hash);
                    format!(
                        "Buy transaction {:#?} failed \nToken price: {:#?} ETH\nVolume: {:#?} ETH",
                        buy_tx_hash,
                        token_price,
                        format_ether(buy_amount)
                    )
                } else {
                    println!("[BuyService] tx success {:?}", buy_tx_hash);
                    wallet_context_mut.eth_balance -= buy_amount;
                    format!(
                        "Buy transaction {:#?} success \nToken price: {:#?} ETH\nVolume: {:#?} ETH",
                        buy_tx_hash,
                        token_price,
                        format_ether(buy_amount)
                    )
                };
                message_transport_service.send_message(message).await?;
                wallet_context_mut.nonce += U256::one();

                Ok(true)
            }
            Err(err) => {
                println!("reset wallet context because of {:?}", err);

                let token_contract =
                    MemeTokenAbigen::new(self.env.token_address, self.http_provider.clone());
                let balance_of = token_contract.balance_of(wallet_context_mut.address);
                let (token_balance, eth_balance, nonce) = tokio::join!(
                    balance_of.call(),
                    self.http_provider
                        .get_balance(wallet_context_mut.address, None),
                    self.http_provider
                        .get_transaction_count(wallet_context_mut.address, None)
                );
                let token_balance = token_balance?;
                let eth_balance = eth_balance?;
                let nonce = nonce?;
                wallet_context_mut.token_balance = token_balance;
                wallet_context_mut.eth_balance = eth_balance;
                wallet_context_mut.nonce = nonce;
                Ok(true)
            }
        }
    }

    fn load_wallet(&self, index: u32) -> Result<LocalWallet, WalletError> {
        let wallet = load_mnemonic_wallet(&self.buyer_mnemonic, index)?;
        let wallet = wallet.with_chain_id(self.env.chain_id.as_u64());
        Ok(wallet)
    }

    pub async fn get_signed_buy_txs(&self) -> anyhow::Result<Vec<(Bytes, usize, U256)>> {
        let system_wallets = compute_system_wallets(
            &self.buyer_mnemonic,
            self.buyer_wallets_count,
            &self.env.token_address,
            self.http_provider.clone(),
        )
        .await?;

        let mut wallet_configs: Vec<(usize, Address)> = Vec::new(); // (wallet_index, wallet_address)

        for (wallet_address, wallet) in system_wallets.clone() {
            let _wallet_lock = match wallet.try_write() {
                Ok(wallet_lock) => wallet_lock,
                Err(_) => continue,
            };

            wallet_configs.push((_wallet_lock.index as usize, wallet_address));
        }

        let mut signed_txs: Vec<(Bytes, usize, U256)> = Vec::new();

        let (pair_address, _) = self
            .router_service
            .get_pair_address(&self.env.token_address, &self.weth_address, true)
            .await?;

        for (wallet_index, wallet_address) in wallet_configs {
            let Some(wallet_context) = system_wallets.get(&wallet_address) else {
                continue;
            };

            let wallet_context = wallet_context.write().await;
            let wallet = self.load_wallet(wallet_context.index)?;

            if wallet_context.eth_balance <= self.buyer_surplus_balance {
                return Err(anyhow::anyhow!(
                    "Wallet index {:?} surplus bigger than balance",
                    wallet_index
                ));
            }
            let buy_amount = wallet_context.eth_balance - self.buyer_surplus_balance;

            let signed_tx = self
                .router_service
                .construct_buy_token_tx(
                    &wallet,
                    Some(wallet_context.nonce),
                    buy_amount,
                    &pair_address,
                    false,
                )
                .await?;

            signed_txs.push((signed_tx, wallet_index, wallet_context.nonce));
        }

        Ok(signed_txs)
    }
}
