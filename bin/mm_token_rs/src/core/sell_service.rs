use anyhow::anyhow;
use cached::Cached;
use cached::TimedCache;
use ethers::abi::AbiParser;
use ethers::abi::Bytes;
use ethers::abi::Tokenizable;
use ethers::utils::hex;
use ethers::{
    contract::parse_log,
    providers::{Http, Middleware, Provider},
    signers::{LocalWallet, Signer, WalletError},
    types::{Address, BlockNumber, Filter, H256, U256, U64},
    utils::{format_units, parse_ether},
};
use futures::future::join_all;
use futures::FutureExt;
use mm_token_utils::constants::ERouter;
use mm_token_utils::constants::UNISWAP3_ROUTERS;
use mm_token_utils::constants::UNIVERSAL_ROUTERS;
use mm_token_utils::utils::universal_decode;
use mm_token_utils::utils::SwapUniversalRouterInfo;
use mm_token_utils::{
    abi::{IUniswapV2PairAbigenEvents, MemeTokenAbigen},
    constants::{UNISWAP2_ROUTERS, WRAPPED_NATIVE_TOKENS, ZERO_ADDRESS},
    env::get_env,
    utils::{compute_transaction_hash, load_mnemonic_wallet},
};
use provider_utils::{http_providers::HttpProviders, ws_providers::WsProviders};
use rand::{seq::SliceRandom, Rng};
use std::{
    collections::HashMap,
    sync::{atomic::Ordering, Arc},
    time::Duration,
};
use tokio::{
    sync::{Mutex, RwLock},
    time::timeout,
};
use tokio_stream::StreamExt;

use crate::routers::RouterService;
use crate::utils::compute_all_system_wallets;
use crate::{
    constants::Env,
    core::MessageTransportService,
    types::TokenInfo,
    utils::{compute_system_wallets, WalletContext},
};

#[derive(Debug, Clone)]
pub struct SellService {
    env: Env,
    http_provider: Arc<Provider<Http>>,
    weth_address: Address,
    token_info: TokenInfo,
    provider_index: Arc<RwLock<usize>>,
    seller_mnemonic: String,
    seller_wallets_count: u32,
    buyer_mnemonic: String,
    buyer_wallets_count: u32,
    auto_buyer_mnemonic: String,
    auto_buyer_wallets_count: u32,
    auto_sell_volume_threshold: U256,
    auto_sell_min_percent: u32,
    auto_sell_max_percent: u32,
    buy_tax: f32,
    router_service: RouterService,
    auto_buyer_system_wallets: Vec<Address>,
    buyer_system_wallets: Vec<Address>,
    seller_system_wallets: HashMap<Address, Arc<RwLock<WalletContext>>>,
    market_maker_system_wallets: Vec<Address>,
}

impl SellService {
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

        let buy_tax: f32 = get_env("TOKEN_BUY_TAX", None).parse().unwrap();
        Self {
            env: env.clone(),
            weth_address: weth.address,
            http_provider: http_provider.clone(),
            token_info: TokenInfo::default(),
            provider_index,
            seller_mnemonic: get_env("SELLER_MNEMONIC", None),
            seller_wallets_count: get_env("SELLER_WALLETS_COUNT", None).parse().unwrap(),
            buyer_mnemonic: get_env("BUYER_MNEMONIC", None),
            buyer_wallets_count: get_env("BUYER_WALLETS_COUNT", None).parse().unwrap(),
            auto_buyer_mnemonic: get_env("AUTO_BUYER_MNEMONIC", None),
            auto_buyer_wallets_count: get_env("AUTO_BUYER_WALLETS_COUNT", None).parse().unwrap(),
            auto_sell_volume_threshold: parse_ether(get_env("AUTO_SELL_VOLUME_THRESHOLD", None))
                .unwrap(),
            buy_tax,
            auto_sell_min_percent: get_env("AUTO_SELL_MIN_PERCENT", None).parse().unwrap(),
            auto_sell_max_percent: get_env("AUTO_SELL_MAX_PERCENT", None).parse().unwrap(),
            router_service: RouterService::new(env, gas_price, http_provider),
            auto_buyer_system_wallets: Vec::new(),
            buyer_system_wallets: Vec::new(),
            seller_system_wallets: HashMap::new(),
            market_maker_system_wallets: Vec::new(),
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

        (
            self.auto_buyer_system_wallets,
            self.buyer_system_wallets,
            _,
            self.market_maker_system_wallets,
        ) = compute_all_system_wallets(
            &self.auto_buyer_mnemonic,
            self.auto_buyer_wallets_count,
            &self.buyer_mnemonic,
            self.buyer_wallets_count,
            &self.seller_mnemonic,
            self.seller_wallets_count,
        )
        .await?;

        self.seller_system_wallets = compute_system_wallets(
            &self.auto_buyer_mnemonic,
            self.auto_buyer_wallets_count,
            &self.env.token_address,
            self.http_provider.clone(),
        )
        .await?;

        Ok(())
    }

    pub async fn start_event_mode(
        &self,
        tx_hashes_cache: Arc<Mutex<TimedCache<H256, bool>>>,
    ) -> anyhow::Result<()> {
        let message_transport_service = MessageTransportService::new();
        let message = "Sell service event mode have been launch".to_string();
        message_transport_service.send_message(message).await?;

        let pair_addresses = self
            .router_service
            .get_all_pair_addresses(&self.env.token_address, &self.weth_address)
            .await?;

        let mut futures = Vec::new();
        for pair_address in pair_addresses {
            log::info!("initialized, token-weth pair is {:?}", pair_address);
            let auto_buy_service = self.clone();
            let tx_hashes_cache = tx_hashes_cache.clone();

            futures.push(
                tokio::spawn(async move {
                    let _ = auto_buy_service
                        .detect_buy_tx(pair_address, tx_hashes_cache)
                        .await;
                })
                .boxed(),
            )
        }
        join_all(futures).await;

        Ok(())
    }

    pub async fn start_mempool_mode(
        self,
        tx_hashes_cache: Arc<Mutex<TimedCache<H256, bool>>>,
    ) -> anyhow::Result<()> {
        let message_transport_service = MessageTransportService::new();
        let message = "Sell service mempool mode have been launch".to_string();
        message_transport_service.send_message(message).await?;

        let get_ws_providers =
            WsProviders::get_ws_providers(&self.env.listen_network, false).await?;

        let stream_mempool = get_ws_providers[0].subscribe_pending_txs().await.unwrap();
        let mut stream_mempool = stream_mempool.transactions_unordered(128).fuse();

        let Some(universal_router_address) = UNIVERSAL_ROUTERS.get(&self.env.listen_network) else {
            panic!(
                "UNIVERSAL_ROUTERS not found in {:?}",
                self.env.listen_network
            );
        };
        if *universal_router_address == *ZERO_ADDRESS {
            log::warn!(
                "UNIVERSAL_ROUTERS not support in {:?}",
                self.env.listen_network
            );
        }

        let Some(uniswapv2_router_address) = UNISWAP2_ROUTERS.get(&self.env.listen_network) else {
            panic!(
                "UNISWAP2_ROUTERS not found in {:?}",
                self.env.listen_network
            );
        };
        if *uniswapv2_router_address == *ZERO_ADDRESS {
            log::warn!(
                "UNISWAP2_ROUTERS not support in {:?}",
                self.env.listen_network
            );
        }

        let Some(uniswapv3_router_address) = UNISWAP3_ROUTERS.get(&self.env.listen_network) else {
            panic!(
                "UNISWAP3_ROUTERS not found in {:?}",
                self.env.listen_network
            );
        };
        if *uniswapv3_router_address == *ZERO_ADDRESS {
            log::warn!(
                "UNISWAP3_ROUTERS not support in {:?}",
                self.env.listen_network
            );
        }

        loop {
            if self.env.exit.load(Ordering::Relaxed) {
                return Err(anyhow!(
                    "[SellService.start_event_mode] exit={:?}",
                    self.env.exit
                ));
            }
            let Some(result) = stream_mempool.next().await else {
                break;
            };
            let tx = result.unwrap_or_default();

            // let tx_hash_test = H256::from_str(
            //     "0x0cb237845aa015136cbb0ece26c4a1aadf0e95b7d27ffb1d911f2afb4dec463c",
            // )
            // .unwrap();
            // let tx = self
            //     .http_provider
            //     .get_transaction(tx_hash_test)
            //     .await?
            //     .unwrap();

            let is_swap_tx_universal_router_matched =
                tx.input.starts_with(&hex::decode("0x3593564c").unwrap()); // execute(bytes commands,bytes[] inputs,uint256 deadline) methodId
            let is_buy_tx_uniswap_v2_matched =
                tx.input.starts_with(&hex::decode("0xb6f9de95").unwrap()); // swapExactETHForTokensSupportingFeeOnTransferTokens methodId
            let is_buy_tx_uniswap_v3_matched =
                tx.input.starts_with(&hex::decode("0x04e45aaf").unwrap()); // exactInputSingle(ExactInputSingleParams memory params) methodId

            let trigger_mempool_router: ERouter;
            let pool_address: Address;
            let pool_v3_fee_tier: u32;
            let token_price: f64;

            // check universal router
            if is_swap_tx_universal_router_matched && tx.to == Some(*universal_router_address) {
                let sig = "function execute(bytes,bytes[],uint256) external payable";
                let func = AbiParser::default().parse_function(sig)?;
                let decoded_data = func.decode_input(&tx.input[4..])?;
                let decode_command =
                    Bytes::from_token(decoded_data.first().unwrap().clone()).unwrap();
                let input_data = decoded_data.get(1).unwrap().clone().into_array().unwrap();

                let mut is_buy_tx_universal_matched: bool = false;
                for index in 0..decode_command.len() {
                    let command = decode_command[index];
                    let input = &input_data[index];
                    let decode_input = Bytes::from_token(input.clone()).unwrap();

                    let swap_info: SwapUniversalRouterInfo =
                        universal_decode(command, decode_input);

                    if !swap_info.path.is_empty() {
                        let from_token = swap_info.path[0];
                        let to_token = swap_info.path[1];
                        if from_token == self.weth_address && to_token == self.token_info.address {
                            log::info!("[AutoSell] from universal router buy tx: {:#?}", tx.hash);
                            is_buy_tx_universal_matched = true;
                        }
                    }
                }

                if !is_buy_tx_universal_matched {
                    continue;
                }

                pool_address = self
                    .router_service
                    .get_pair_address_by_router(
                        &self.env.token_address,
                        &self.weth_address,
                        true,
                        None,
                        ERouter::UniversalRouters,
                    )
                    .await?
                    .0;
                token_price = self
                    .router_service
                    .get_token_native_price(ERouter::UniversalRouters, pool_address)
                    .await?;
                trigger_mempool_router = ERouter::UniversalRouters;
            }
            // check uniswapv2 router
            else if is_buy_tx_uniswap_v2_matched && tx.to == Some(*uniswapv2_router_address) {
                let sig = "function swapExactETHForTokensSupportingFeeOnTransferTokens(uint256,address[],address,uint256) external payable";
                let func = AbiParser::default().parse_function(sig)?;
                let decoded_data: Vec<_> = func.decode_input(&tx.input[4..])?;
                let vec_token: Vec<Address> =
                    Vec::from_token(decoded_data.get(1).unwrap().clone()).unwrap(); // [0]: WETH, [1]: token
                let buy_token = vec_token[1];

                if buy_token != self.token_info.address {
                    log::info!("self.token_info.address: {:#?}", self.token_info.address);
                    log::info!("vec_token: {:#?}", vec_token);
                    log::warn!("tx is not from token");
                    continue;
                }

                pool_address = self
                    .router_service
                    .get_pair_address_by_router(
                        &self.env.token_address,
                        &self.weth_address,
                        true,
                        None,
                        ERouter::Uniswap2Routers,
                    )
                    .await?
                    .0;
                token_price = self
                    .router_service
                    .get_token_native_price(ERouter::Uniswap2Routers, pool_address)
                    .await?;
                trigger_mempool_router = ERouter::Uniswap2Routers;
            }
            // check uniswapv3 router
            else if is_buy_tx_uniswap_v3_matched && tx.to == Some(*uniswapv3_router_address) {
                let sig = "function exactInputSingle(address,address,uint24,address,uint256,uint256,uint160) external payable override";
                let func = AbiParser::default().parse_function(sig)?;
                let decoded_data: Vec<_> = func.decode_input(&tx.input[4..])?;

                let buy_token = Address::from_token(decoded_data.get(1).unwrap().clone()).unwrap();
                if buy_token != self.token_info.address {
                    continue;
                }

                pool_v3_fee_tier = u32::from_token(decoded_data.get(2).unwrap().clone()).unwrap();

                pool_address = self
                    .router_service
                    .get_pair_address_by_router(
                        &self.env.token_address,
                        &self.weth_address,
                        true,
                        Some(pool_v3_fee_tier),
                        ERouter::Uniswap3Routers,
                    )
                    .await?
                    .0;
                token_price = self
                    .router_service
                    .get_token_native_price(ERouter::Uniswap3Routers, pool_address)
                    .await?;
                trigger_mempool_router = ERouter::Uniswap3Routers;
            } else {
                // not match with all of routers
                continue;
            }

            let transaction_hash = tx.hash;
            let transaction_value = tx.value;

            if transaction_value < self.auto_sell_volume_threshold {
                continue;
            }

            // if self.buyer_system_wallets.contains(&tx.from) {
            //     log::warn!(
            //         "tx {:?} from buyer system wallet {:?}, skip",
            //         tx.hash,
            //         tx.from
            //     );
            //     continue;
            // }
            // if self.auto_buyer_system_wallets.contains(&tx.from) {
            //     log::warn!(
            //         "tx {:?} from auto buyer system wallet {:?}, skip",
            //         tx.hash,
            //         tx.from
            //     );
            //     continue;
            // }
            // if self.seller_system_wallets.contains_key(&tx.from) {
            //     log::warn!(
            //         "tx {:?} from seller system wallet {:?}, skip",
            //         tx.hash,
            //         tx.from
            //     );
            //     continue;
            // }
            // if self.market_maker_system_wallets.contains(&tx.from) {
            //     log::warn!(
            //         "tx {:?} from market maker system wallet {:?}, skip",
            //         tx.hash,
            //         tx.from
            //     );
            //     continue;
            // }

            // set tx trigger to cache
            let mut tx_hashes_cache = tx_hashes_cache.lock().await;
            tx_hashes_cache.cache_set(tx.hash, true);
            drop(tx_hashes_cache);

            println!(
                "token_price: {:#?},transaction_value: {:#?}, pool_address: {:#?}",
                token_price, transaction_value, pool_address
            );

            self.process_trigger_sell(
                &self.seller_system_wallets,
                transaction_value,
                transaction_hash,
                true,
                &pool_address,
                trigger_mempool_router,
                token_price,
            )
            .await?;
        }

        Ok(())
    }

    async fn detect_buy_tx(
        mut self,
        pair_address: Address,
        tx_hashes_cache: Arc<Mutex<TimedCache<H256, bool>>>,
    ) -> anyhow::Result<()> {
        // detect weth transfer from user to pair (user buy token from pair)
        // conflict with add_liquidity, but we accept it
        let erc20_transfer_filter = Filter::new()
            .from_block(BlockNumber::Latest)
            .event("Transfer(address,address,uint256)")
            .topic2(H256::from(pair_address))
            .address(self.weth_address);

        let mut receiver = WsProviders::subscribe_logs_stream(
            &self.env.listen_network,
            erc20_transfer_filter,
            false,
        )
        .await?;

        // assume that tx is success because there is an Transfer event
        loop {
            if self.env.exit.load(Ordering::Relaxed) {
                return Err(anyhow!(
                    "[SellService.start_event_mode] exit={:?}",
                    self.env.exit
                ));
            }

            let Ok(next_value) = timeout(Duration::from_millis(100), receiver.recv()).await else {
                continue;
            };
            let Ok(log) = next_value else {
                break;
            };

            let transaction_hash = log.transaction_hash.unwrap_or_default();

            tokio::time::sleep(Duration::from_secs(1)).await; // wait for mempool to cache first
            let mut tx_hashes_cache = tx_hashes_cache.lock().await;
            if tx_hashes_cache.cache_get(&transaction_hash).is_some() {
                log::warn!(
                    "Meet this tx hash before from mempool mode: {:#?}",
                    transaction_hash
                );
                drop(tx_hashes_cache);
                continue;
            }

            // get healthy provider
            self.http_provider = Arc::new(
                HttpProviders::get_provider(
                    &self.env.listen_network,
                    false,
                    self.provider_index.clone(),
                )
                .await?,
            );

            let Ok(IUniswapV2PairAbigenEvents::TransferFilter(decoded)) = parse_log(log) else {
                continue;
            };
            if decoded.value < self.auto_sell_volume_threshold {
                continue;
            }
            let tx = self.http_provider.get_transaction(transaction_hash).await?;
            let Some(tx) = tx else {
                log::warn!("cannot fetch tx {:?} from fullnode", transaction_hash);
                continue;
            };
            // if self.buyer_system_wallets.contains(&tx.from) {
            //     log::warn!(
            //         "tx {:?} from buyer system wallet {:?}, skip",
            //         tx.hash,
            //         tx.from
            //     );
            //     continue;
            // }
            // if self.auto_buyer_system_wallets.contains(&tx.from) {
            //     log::warn!(
            //         "tx {:?} from auto buyer system wallet {:?}, skip",
            //         tx.hash,
            //         tx.from
            //     );
            //     continue;
            // }
            // if self.seller_system_wallets.contains_key(&tx.from) {
            //     log::warn!(
            //         "tx {:?} from seller system wallet {:?}, skip",
            //         tx.hash,
            //         tx.from
            //     );
            //     continue;
            // }
            // if self.market_maker_system_wallets.contains(&tx.from) {
            //     log::warn!(
            //         "tx {:?} from market maker system wallet {:?}, skip",
            //         tx.hash,
            //         tx.from
            //     );
            //     continue;
            // }

            let token_price = self
                .router_service
                .get_token_native_price(self.router_service.active_router, pair_address)
                .await?;

            self.process_trigger_sell(
                &self.seller_system_wallets,
                decoded.value,
                transaction_hash,
                false,
                &pair_address,
                self.router_service.active_router,
                token_price,
            )
            .await?;
        }

        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    async fn process_trigger_sell(
        &self,
        system_wallets: &HashMap<Address, Arc<RwLock<WalletContext>>>,
        tx_value: U256,
        trigger_tx_hash: H256,
        is_from_mempool: bool,
        pair_address: &Address,
        active_router: ERouter,
        token_price: f64,
    ) -> anyhow::Result<()> {
        let message_transport_service = MessageTransportService::new();

        if is_from_mempool {
            log::info!(
                "[SellService] trigger sell from mempool mode for buy tx {:?}",
                trigger_tx_hash,
            );
            let message = format!(
                "[SellService] trigger sell from mempool mode for buy tx {:?}",
                trigger_tx_hash
            );
            message_transport_service.send_message(message).await?;
        } else {
            log::info!(
                "[SellService] trigger sell from event mode for buy tx {:?}",
                trigger_tx_hash,
            );
            let message = format!(
                "[SellService] trigger sell from event mode for buy tx {:?}",
                trigger_tx_hash
            );
            message_transport_service.send_message(message).await?;
        }

        let mut wallet_configs: Vec<(Address, U256)> = Vec::new(); // (wallet_index, token_sell_amount)
        let mut the_chosen_ones: Vec<Address> = Vec::new();
        let tx_sell_amount = self
            .router_service
            .get_amount_out(
                active_router,
                pair_address,
                true,
                Some(&self.weth_address),
                Some(&self.env.token_address),
                tx_value,
                self.buy_tax,
            )
            .await?;
        let random_sell_percent =
            rand::thread_rng().gen_range(self.auto_sell_min_percent..=self.auto_sell_max_percent);
        let mut total_sell_amount =
            tx_sell_amount * U256::from(random_sell_percent) / U256::from(100);

        for wallet in system_wallets.values() {
            if total_sell_amount == U256::zero() {
                break;
            }
            // try write, if wallet is in used, skip it
            let Ok(wallet_context) = wallet.try_write() else {
                continue;
            };
            if wallet_context.token_balance == U256::zero() {
                continue;
            }
            if wallet_context.token_balance <= total_sell_amount {
                wallet_configs.push((wallet_context.address, wallet_context.token_balance));
                total_sell_amount -= wallet_context.token_balance;
                continue;
            }

            the_chosen_ones.push(wallet_context.address);
        }
        if total_sell_amount > U256::zero() {
            let the_chosen_one: Option<&ethers::types::H160> =
                the_chosen_ones.choose(&mut rand::thread_rng());
            if let Some(the_chosen_one) = the_chosen_one {
                wallet_configs.push((*the_chosen_one, total_sell_amount));
            } else {
                log::warn!(
                    "cannot find any wallet for total_sell_amount {:?}",
                    total_sell_amount
                );
                let message = format!(
                    "Cannot find any wallet for total_sell_amount {:#?} {:#?}",
                    format_units(total_sell_amount, self.token_info.decimals as usize)?,
                    self.token_info.symbol
                );
                message_transport_service.send_message(message).await?;
            }
        }

        for (wallet_address, sell_amount) in wallet_configs {
            let Some(wallet_context) = system_wallets.get(&wallet_address) else {
                continue;
            };
            let wallet_context = wallet_context.clone();
            let sell_service = self.clone();
            let pair_address_clone = *pair_address;
            tokio::spawn(async move {
                let _ = sell_service
                    .sell(
                        trigger_tx_hash,
                        wallet_context,
                        sell_amount,
                        token_price,
                        &pair_address_clone,
                    )
                    .await;
            });
        }

        Ok(())
    }

    async fn sell(
        &self,
        trigger_tx_hash: H256,
        wallet_context: Arc<RwLock<WalletContext>>,
        sell_amount: U256,
        token_price: f64,
        pair_address: &Address,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        let message_transport_service = MessageTransportService::new();

        let mut wallet_context_mut = wallet_context.write().await;
        let wallet = self.load_wallet(wallet_context_mut.index)?;

        let signed_sell_tx = match self
            .router_service
            .construct_sell_token_tx(
                &wallet,
                Some(wallet_context_mut.nonce),
                sell_amount,
                pair_address,
                true,
            )
            .await
        {
            Ok(signed_buy_tx) => signed_buy_tx,
            Err(err) => {
                log::warn!("[SellService] try_sell {:?}", err);
                return Ok(true);
            }
        };

        let sell_tx_hash = compute_transaction_hash(&signed_sell_tx);
        log::info!(
            "[SellService] wallet index {:?} address {:?} selling {:?} for buy tx {:?}",
            wallet_context_mut.index,
            wallet.address(),
            sell_tx_hash,
            trigger_tx_hash,
        );

        let pending_tx = self
            .http_provider
            .send_raw_transaction(signed_sell_tx)
            .await;
        log::info!(
            "[SellService] pending tx sell from wallet index {:#?} result {:?}",
            wallet_context_mut.index,
            pending_tx
        );
        match pending_tx {
            Ok(pending_tx) => {
                let tx_receipt = pending_tx.await?.ok_or(anyhow!("Cannot find tx_receipt"))?;
                let message = if tx_receipt.status == Some(U64::zero()) {
                    log::warn!("Sell transaction {:#?} failed", tx_receipt.transaction_hash);
                    format!(
                        "Sell transaction {:#?} failed \nToken price: {:#?} ETH\nVolume: {:#?} {:#?}",
                        tx_receipt.transaction_hash,
                        token_price,
                        format_units(sell_amount, self.token_info.decimals as usize)?,
                        self.token_info.symbol
                    )
                } else {
                    log::info!("[SellService] tx success {:?}", tx_receipt.transaction_hash);
                    wallet_context_mut.token_balance -= sell_amount;
                    format!(
                        "Sell transaction {:#?} success \nToken price: {:#?} ETH\nVolume: {:#?} {:#?}",
                        tx_receipt.transaction_hash,
                        token_price,
                        format_units(sell_amount, self.token_info.decimals as usize)?,
                        self.token_info.symbol
                    )
                };
                message_transport_service.send_message(message).await?;
                wallet_context_mut.nonce += U256::one();
                Ok(true)
            }
            Err(err) => {
                log::warn!("reset wallet context because of {:?}", err);
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
        let wallet = load_mnemonic_wallet(&self.seller_mnemonic, index)?;
        let wallet = wallet.with_chain_id(self.env.chain_id.as_u64());
        Ok(wallet)
    }
}

#[derive(Debug, Default)]
pub struct IsPacmanSellResult {
    pub is_sell: bool,
    pub mm_token_sell_amount: U256,
}
