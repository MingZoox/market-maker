use std::sync::Arc;
use std::{str::FromStr, time::Duration};

use chrono::Utc;
use ethers::types::{H256, U64};
use ethers::utils::keccak256;
use ethers::{
    providers::{Http, Middleware, Provider},
    signers::{LocalWallet, Signer, WalletError},
    types::{transaction::eip2718::TypedTransaction, Address, Bytes, TransactionRequest, U256},
    utils::parse_ether,
};
use mm_token_toolkit::bundler::{BloxrouteConfig, Bundler};
use mm_token_utils::{
    abi::UniswapV2Router02Abigen,
    constants::{UNISWAP2_ROUTERS, WRAPPED_NATIVE_TOKENS},
    env::get_env,
    utils::{load_mnemonic_wallet, to_signed_tx},
};
use provider_utils::constants::DESERIALIZATION_ERROR_MSG;
use provider_utils::http_providers::HttpProviders;
use rand::Rng;
use tokio::time::timeout;
use tokio::{sync::RwLock, time};
use tokio_stream::wrappers::IntervalStream;
use tokio_stream::StreamExt;

use crate::{constants::Env, utils::get_bloxroute_tip_fee};

pub struct MevBuyService {
    env: Env,
    buyer_mnemonic: String,
    buyer_wallets_count: u32,
    buyer_surplus_balance: U256,
    tip_pk: String,
    tip_eth_amount: U256,
    activate_pk: String,
    open_trading_address: Address,
    open_trading_method: String,
    http_provider: Arc<Provider<Http>>,
    gas_price: Arc<RwLock<U256>>,
    provider_index: Arc<RwLock<usize>>,
    bundler: Bundler,
    weth_address: Address,
    uniswapv2_router_address: Address,
    bloxroute_tip_address: Address,
}

impl MevBuyService {
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
        let bundler = Bundler::new(
            env.listen_network,
            BloxrouteConfig {
                relay_url: get_env("BLOXROUTE_RELAY_URL", None),
                authorization_key: get_env("BLOXROUTE_AUTH_KEY", None),
            },
        );
        Self {
            buyer_mnemonic: get_env("BUYER_MNEMONIC", None),
            buyer_surplus_balance: parse_ether(get_env("BUYER_SURPLUS_BALANCE", None)).unwrap(),
            buyer_wallets_count: get_env("BUYER_WALLETS_COUNT", None).parse().unwrap(),
            tip_pk: get_env("TIP_PK", None),
            tip_eth_amount: parse_ether(get_env("TIP_ETH_AMOUNT", None)).unwrap(),
            activate_pk: get_env("ACTIVATE_PK", None),
            open_trading_address: Address::from_str(&get_env("OPEN_TRADING_ADDRESS", None))
                .unwrap(),
            open_trading_method: get_env("OPEN_TRADING_METHOD", None),
            http_provider,
            uniswapv2_router_address: *uniswapv2_router_address,
            env,
            gas_price,
            provider_index,
            weth_address: weth.address,
            bundler,
            bloxroute_tip_address: Address::from_str("0x965Df5Ff6116C395187E288e5C87fb96CfB8141c")
                .unwrap(),
        }
    }

    pub async fn start(mut self) -> anyhow::Result<()> {
        let mut latest_block = self.http_provider.get_block_number().await?;
        let mut first_tx_hash_in_batch: Option<H256> = None;
        let mut stream = IntervalStream::new(time::interval(Duration::from_millis(500)));
        loop {
            if self.env.exit.load(std::sync::atomic::Ordering::Relaxed) {
                break;
            }
            let Ok(_) = timeout(Duration::from_millis(100), stream.next()).await else {
                continue;
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

            let current_block = match self.http_provider.get_block_number().await {
                Ok(current_block) => current_block,
                Err(err) => {
                    if err.to_string().contains(DESERIALIZATION_ERROR_MSG) {
                        continue;
                    }
                    return Err(err.into());
                }
            };

            if current_block == latest_block {
                continue;
            }

            if let Some(tx_hash) = first_tx_hash_in_batch {
                match self.http_provider.get_transaction_receipt(tx_hash).await {
                    Ok(tx_receipt) => {
                        if let Some(tx_receipt) = tx_receipt {
                            if tx_receipt.status == Some(U64::one()) {
                                log::info!("Bundle success: {:?}, exiting", tx_receipt);
                                break;
                            }
                        }
                    }
                    Err(err) => {
                        if err.to_string().contains(DESERIALIZATION_ERROR_MSG) {
                            continue;
                        }
                        return Err(err.into());
                    }
                }
            }

            let tx_hash = match self.mev_snipe(current_block).await {
                Ok(tx_hash) => tx_hash,
                Err(err) => {
                    if err.to_string().contains(DESERIALIZATION_ERROR_MSG) {
                        continue;
                    }
                    return Err(err);
                }
            };
            log::info!("First tx hash in batch: {:?}", tx_hash);
            first_tx_hash_in_batch = Some(tx_hash);
            latest_block = current_block;
        }

        Ok(())
    }

    pub async fn mev_snipe(&self, current_block: U64) -> anyhow::Result<H256> {
        log::info!("Mev sniping block: {:?}", current_block);
        let (tip_tx, activate_tx) = tokio::join!(
            self.compute_tip_tx(self.buyer_wallets_count + 2),
            self.compute_activate_tx()
        );
        let (tip_tx, activate_tx) = (tip_tx?, activate_tx?);
        let first_tx_hash = H256::from_slice(&keccak256(&tip_tx));
        let mut signed_txs = vec![tip_tx, activate_tx];

        let mut jobs = Vec::new();
        for i in 0..self.buyer_wallets_count {
            jobs.push(self.compute_signed_buy_tx(i))
        }
        let signed_buy_txs = futures::future::join_all(jobs).await;
        let signed_buy_txs = signed_buy_txs
            .into_iter()
            .collect::<Result<Vec<Bytes>, _>>()?;
        signed_txs.extend(signed_buy_txs);

        let bundle = self
            .bundler
            .to_bundle(&signed_txs, current_block, current_block + U64::one());
        log::info!("Sending bundle {:?}", bundle);
        let bundle_hashes = self.bundler.send_bundle(&bundle).await?;
        log::info!("Bundle hashes: {:?}", bundle_hashes);

        Ok(first_tx_hash)
    }

    async fn compute_tip_tx(&self, number_of_txs: u32) -> anyhow::Result<Bytes> {
        let wallet = self.load_tip_wallet()?;
        let gas_price = *self.gas_price.read().await;
        let tip_value = get_bloxroute_tip_fee(&self.env.listen_network, number_of_txs);
        let nonce = self
            .http_provider
            .get_transaction_count(wallet.address(), None)
            .await?;
        let tip_gas = self.tip_eth_amount / U256::from(21000);

        let tx = TransactionRequest::new()
            .from(wallet.address())
            .nonce(nonce)
            .to(self.bloxroute_tip_address)
            .value(tip_value)
            .gas_price(gas_price + tip_gas)
            .gas(21000);
        let mut tip_tx = TypedTransaction::Legacy(tx);
        tip_tx.set_chain_id(self.env.chain_id);
        let signed_tx = to_signed_tx(&wallet, &tip_tx).await?;

        Ok(signed_tx)
    }

    async fn compute_activate_tx(&self) -> anyhow::Result<Bytes> {
        let wallet = self.load_activate_wallet()?;
        let gas_price = *self.gas_price.read().await;
        let nonce = self
            .http_provider
            .get_transaction_count(wallet.address(), None)
            .await?;

        let method_id = ethers::utils::id(&self.open_trading_method);
        let tx = TransactionRequest::new()
            .from(wallet.address())
            .nonce(nonce)
            .to(self.open_trading_address)
            .data(Bytes::from(method_id.to_vec()))
            .gas_price(gas_price)
            .gas(500_000);
        let mut activate_tx = TypedTransaction::Legacy(tx);
        activate_tx.set_chain_id(self.env.chain_id);
        let signed_tx = to_signed_tx(&wallet, &activate_tx).await?;

        Ok(signed_tx)
    }

    async fn compute_signed_buy_tx(&self, wallet_index: u32) -> anyhow::Result<Bytes> {
        let wallet = self.load_mev_buy_wallet(wallet_index)?;

        let uniswapv2_router =
            UniswapV2Router02Abigen::new(self.uniswapv2_router_address, self.http_provider.clone());
        let gas_price = *self.gas_price.read().await;
        let nonce = self
            .http_provider
            .get_transaction_count(wallet.address(), None)
            .await?;
        let balance = self
            .http_provider
            .get_balance(wallet.address(), None)
            .await?;
        if balance < self.buyer_surplus_balance {
            return Err(anyhow::anyhow!(
                "Insufficient balance {:?}",
                wallet.address()
            ));
        }
        let random_gas_limit = rand::thread_rng().gen_range(500_000..=550_000); // fixed gas limit

        let deadline = U256::from(Utc::now().timestamp()) + U256::from(120);
        let mut buy_tx: TypedTransaction = uniswapv2_router
            .swap_exact_eth_for_tokens_supporting_fee_on_transfer_tokens(
                U256::one(),
                vec![self.weth_address, self.env.token_address],
                wallet.address(),
                deadline,
            )
            .from(wallet.address())
            .nonce(nonce)
            .gas(random_gas_limit)
            .gas_price(gas_price)
            .value(balance - self.buyer_surplus_balance)
            .legacy()
            .tx;
        buy_tx.set_chain_id(self.env.chain_id);
        let signed_tx = to_signed_tx(&wallet, &buy_tx).await?;

        Ok(signed_tx)
    }

    fn load_tip_wallet(&self) -> Result<LocalWallet, WalletError> {
        let wallet: LocalWallet = self.tip_pk.parse()?;
        Ok(wallet.with_chain_id(self.env.chain_id.as_u64()))
    }

    fn load_activate_wallet(&self) -> Result<LocalWallet, WalletError> {
        let wallet: LocalWallet = self.activate_pk.parse()?;
        Ok(wallet.with_chain_id(self.env.chain_id.as_u64()))
    }

    fn load_mev_buy_wallet(&self, index: u32) -> Result<LocalWallet, WalletError> {
        let wallet = load_mnemonic_wallet(&self.buyer_mnemonic, index)?;
        Ok(wallet.with_chain_id(self.env.chain_id.as_u64()))
    }
}
