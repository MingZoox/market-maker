use std::{
    sync::{atomic::Ordering, Arc},
    time::Duration,
};

use anyhow::anyhow;
use chrono::Utc;
use ethers::{
    contract::parse_log,
    providers::{Http, Middleware, Provider},
    signers::{LocalWallet, Signer, WalletError},
    types::{
        transaction::eip2718::TypedTransaction, Address, BlockNumber, Filter, Log,
        TransactionReceipt, U256, U64,
    },
    utils::parse_ether,
};
use mm_token_utils::{
    abi::{
        IUniswapV2PairAbigen, IUniswapV2PairAbigenEvents, MemeTokenAbigen, UniswapV2Router02Abigen,
    },
    constants::{UNISWAP2_ROUTERS, WRAPPED_NATIVE_TOKENS, ZERO_ADDRESS},
    env::get_env,
    utils::{compute_transaction_hash, load_mnemonic_wallet, to_legacy_tx, to_signed_tx},
};
use provider_utils::{http_providers::HttpProviders, ws_providers::WsProviders};
use tokio::{sync::RwLock, time::timeout};

use crate::constants::Env;

#[derive(Debug, Clone)]
pub struct SnipeService {
    env: Env,
    http_provider: Arc<Provider<Http>>,
    uniswapv2_router_address: Address,
    uniswapv2_factory_address: Address,
    weth_address: Address,
    gas_price: Arc<RwLock<U256>>,
    provider_index: Arc<RwLock<usize>>,
    snipe_mnemonic: String,
    nonce: Arc<RwLock<U256>>,
}

impl SnipeService {
    pub fn new(
        env: Env,
        gas_price: Arc<RwLock<U256>>,
        provider_index: Arc<RwLock<usize>>,
        http_provider: Arc<Provider<Http>>,
    ) -> Self {
        let Some(uniswapv2_router_address) = UNISWAP2_ROUTERS.get(&env.listen_network) else {
            panic!("UNISWAP2_ROUTERS not found in {:?}", env.listen_network);
        };
        let Some(weth) = WRAPPED_NATIVE_TOKENS.get(&env.listen_network) else {
            panic!(
                "WRAPPED_NATIVE_TOKENS not found in {:?}",
                env.listen_network
            );
        };
        let snipe_mnemonic = get_env("SNIPE_MNEMONIC", Some("".to_string()));

        Self {
            env,
            http_provider,
            uniswapv2_router_address: *uniswapv2_router_address,
            uniswapv2_factory_address: *ZERO_ADDRESS,
            weth_address: weth.address,
            gas_price,
            provider_index,
            snipe_mnemonic,
            nonce: Default::default(),
        }
    }

    pub async fn init(&mut self) -> anyhow::Result<()> {
        let router =
            UniswapV2Router02Abigen::new(self.uniswapv2_router_address, self.http_provider.clone());
        let wallet = self.load_snipe_wallet()?;

        let factory = router.factory();
        let (factory, nonce) = tokio::join!(
            factory.call(),
            self.http_provider
                .get_transaction_count(wallet.address(), None)
        );
        let (factory, nonce) = (factory?, nonce?);

        self.uniswapv2_factory_address = factory;
        self.nonce = Arc::new(RwLock::new(nonce));
        log::info!("initialized, factory={:?}, nonce={:?}", factory, nonce);

        Ok(())
    }

    pub async fn start_event_mode(mut self) -> anyhow::Result<()> {
        let mint_filter = Filter::new()
            .from_block(BlockNumber::Latest)
            .event("Mint(address,uint256,uint256)");
        let mut receiver =
            WsProviders::subscribe_logs_stream(&self.env.listen_network, mint_filter, false)
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

            // get healthy provider
            self.http_provider = Arc::new(
                HttpProviders::get_provider(
                    &self.env.listen_network,
                    false,
                    self.provider_index.clone(),
                )
                .await?,
            );

            let snipe_service = self.clone();
            tokio::spawn(async move {
                let _ = snipe_service.process_log(log).await;
            });
        }

        Ok(())
    }

    async fn process_log(&self, log: Log) -> anyhow::Result<()> {
        let pair = IUniswapV2PairAbigen::new(log.address, self.http_provider.clone());
        let Ok(factory_address) = pair.factory().call().await else {
            return Ok(());
        };
        if factory_address != self.uniswapv2_factory_address {
            return Ok(());
        }

        let (token_0, token_1) = (pair.token_0(), pair.token_1());
        let (token_0, token_1) = tokio::join!(token_0.call(), token_1.call());
        let (token_0, token_1) = (token_0?, token_1?);
        let is_weth_token_0 = self.weth_address == token_0;
        let token = if is_weth_token_0 { token_1 } else { token_0 };
        let Ok(IUniswapV2PairAbigenEvents::MintFilter(decoded)) = parse_log(log) else {
            return Ok(());
        };
        let weth_amount: U256 = if is_weth_token_0 {
            decoded.amount_0
        } else {
            decoded.amount_1
        };
        let snipe_eth_min_threshold =
            parse_ether(get_env("SNIPE_ETH_MIN_THRESHOLD", Some("1".to_string()))).unwrap();
        if weth_amount < snipe_eth_min_threshold {
            log::warn!(
                "skip because snipe_eth_min_threshold={:?}",
                snipe_eth_min_threshold
            );
            return Ok(());
        }

        let wallet = self.load_snipe_wallet()?;
        let Some(tx_receipt) = self.snipe(&wallet, token).await? else {
            log::warn!("snipe failed, auto sell not triggered");
            return Ok(());
        };
        let Some(block_number) = tx_receipt.block_number else {
            log::warn!("block number is null, auto sell not triggered");
            return Ok(());
        };

        // auto sell
        let auto_sell_block =
            U64::from_dec_str(&get_env("SNIPE_AUTO_SELL_BLOCK", Some("10".to_string())))?;
        let target_sell_block = block_number + auto_sell_block;
        loop {
            let current_block_number = self.http_provider.get_block_number().await?;
            if current_block_number < target_sell_block {
                tokio::time::sleep(Duration::from_secs(3)).await;
                continue;
            }

            log::info!(
                "auto_sell triggered, token={:?},block_number={:?}",
                token,
                current_block_number
            );
            self.auto_sell(&wallet, token).await?;
            break;
        }

        Ok(())
    }

    async fn auto_sell(&self, wallet: &LocalWallet, token_address: Address) -> anyhow::Result<()> {
        let wallet_address = wallet.address();
        let token_contract =
            MemeTokenAbigen::new(self.env.token_address, self.http_provider.clone());
        let (balance_of, allowance) = (
            token_contract.balance_of(wallet_address),
            token_contract.allowance(wallet_address, self.uniswapv2_router_address),
        );
        let (token_balance, allowance) = tokio::join!(balance_of.call(), allowance.call());
        let (token_balance, allowance) = (token_balance?, allowance?);
        if token_balance == U256::zero() {
            log::info!("token_balance = 0, exited");
            return Ok(());
        }

        if allowance == U256::zero() {
            log::info!("approving token");
            self.approve(wallet, token_address).await?;
        }
        self.sell(wallet, token_address, token_balance).await?;
        log::info!("selling token");

        Ok(())
    }

    async fn sell(
        &self,
        wallet: &LocalWallet,
        token_address: Address,
        sell_amount: U256,
    ) -> anyhow::Result<()> {
        let mut nonce_mut = self.nonce.write().await;
        let gas_price = *self.gas_price.read().await;
        let uniswapv2_router =
            UniswapV2Router02Abigen::new(self.uniswapv2_router_address, self.http_provider.clone());

        let deadline = U256::from(Utc::now().timestamp()) + U256::from(60);
        let mut tx = uniswapv2_router
            .swap_exact_tokens_for_eth_supporting_fee_on_transfer_tokens(
                sell_amount,
                U256::one(),
                vec![token_address, self.weth_address],
                wallet.address(),
                deadline,
            )
            .tx;
        tx.set_chain_id(self.env.chain_id);
        tx.set_from(wallet.address());
        tx.set_nonce(*nonce_mut);
        tx.set_gas(U256::from(500_000));
        tx.set_gas_price(gas_price);

        let snipe_tx = to_legacy_tx(tx);
        let signed_tx = to_signed_tx(wallet, &snipe_tx).await?;
        let tx_hash = compute_transaction_hash(&signed_tx);
        log::info!(
            "[SnipeService] wallet index {:?} address {:?} auto_selling {:?}",
            0,
            wallet.address(),
            tx_hash,
        );
        let tx_receipt = self
            .http_provider
            .send_raw_transaction(signed_tx)
            .await?
            .await?;
        let Some(tx_receipt) = tx_receipt else {
            log::warn!("auto_sell failed");
            return Ok(());
        };
        log::info!(
            "[SnipeService] wallet index {:?} address {:?} auto_sell done {:?}",
            0,
            wallet.address(),
            tx_receipt.transaction_hash,
        );
        *nonce_mut += U256::one();

        Ok(())
    }

    async fn approve(&self, wallet: &LocalWallet, token_address: Address) -> anyhow::Result<()> {
        let mut nonce_mut = self.nonce.write().await;
        let token = IUniswapV2PairAbigen::new(token_address, self.http_provider.clone());
        let gas_price = *self.gas_price.read().await;

        let mut tx: TypedTransaction = token.approve(self.uniswapv2_router_address, U256::MAX).tx;
        tx.set_chain_id(self.env.chain_id);
        tx.set_from(wallet.address());
        tx.set_nonce(*nonce_mut);
        tx.set_gas(U256::from(500_000));
        tx.set_gas_price(gas_price);

        let snipe_tx = to_legacy_tx(tx);
        let signed_tx = to_signed_tx(wallet, &snipe_tx).await?;
        let tx_hash = compute_transaction_hash(&signed_tx);
        log::info!(
            "[SnipeService] wallet index {:?} address {:?} approving {:?}",
            0,
            wallet.address(),
            tx_hash,
        );
        let tx_receipt = self
            .http_provider
            .send_raw_transaction(signed_tx)
            .await?
            .await?;
        let Some(tx_receipt) = tx_receipt else {
            log::warn!("approve failed");
            return Ok(());
        };
        log::info!(
            "[SnipeService] wallet index {:?} address {:?} approve done {:?}",
            0,
            wallet.address(),
            tx_receipt.transaction_hash,
        );
        *nonce_mut += U256::one();

        Ok(())
    }

    async fn snipe(
        &self,
        wallet: &LocalWallet,
        token: Address,
    ) -> anyhow::Result<Option<TransactionReceipt>> {
        let uniswapv2_router =
            UniswapV2Router02Abigen::new(self.uniswapv2_router_address, self.http_provider.clone());
        let gas_price = *self.gas_price.read().await;
        let snipe_eth_amount =
            parse_ether(get_env("SNIPE_ETH_AMOUNT", Some("0".to_string()))).unwrap();

        let mut nonce_mut = self.nonce.write().await;
        let deadline = U256::from(Utc::now().timestamp()) + U256::from(60);
        let mut snipe_tx: TypedTransaction = uniswapv2_router
            .swap_exact_eth_for_tokens_supporting_fee_on_transfer_tokens(
                U256::one(),
                vec![self.weth_address, token],
                wallet.address(),
                deadline,
            )
            .tx;
        snipe_tx.set_chain_id(self.env.chain_id);
        snipe_tx.set_from(wallet.address());
        snipe_tx.set_nonce(*nonce_mut);
        snipe_tx.set_value(snipe_eth_amount);
        snipe_tx.set_gas(U256::from(500_000));
        snipe_tx.set_gas_price(gas_price);
        let snipe_tx = to_legacy_tx(snipe_tx);
        let signed_snipe_tx = to_signed_tx(wallet, &snipe_tx).await?;
        let snipe_tx_hash = compute_transaction_hash(&signed_snipe_tx);
        log::info!(
            "[SnipeService] wallet index {:?} address {:?} sniping {:?}",
            0,
            wallet.address(),
            snipe_tx_hash,
        );
        let tx_receipt = self
            .http_provider
            .send_raw_transaction(signed_snipe_tx)
            .await?
            .await?;
        let Some(tx_receipt) = tx_receipt else {
            log::warn!("snipe failed");
            return Ok(None);
        };
        log::info!(
            "[SnipeService] wallet index {:?} address {:?} snipe done {:?}",
            0,
            wallet.address(),
            tx_receipt.transaction_hash,
        );
        *nonce_mut += U256::one();

        Ok(Some(tx_receipt))
    }

    fn load_snipe_wallet(&self) -> Result<LocalWallet, WalletError> {
        self.load_mnemonic_wallet(&self.snipe_mnemonic, 0)
    }

    fn load_mnemonic_wallet(&self, mnemonic: &str, index: u32) -> Result<LocalWallet, WalletError> {
        let wallet = load_mnemonic_wallet(mnemonic, index)?;
        let wallet = wallet.with_chain_id(self.env.chain_id.as_u64());
        Ok(wallet)
    }
}
