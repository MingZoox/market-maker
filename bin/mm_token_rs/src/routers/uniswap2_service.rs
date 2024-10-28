use bigdecimal::BigDecimal;
use chrono::Utc;
use ethers::{
    providers::{Http, Middleware, Provider},
    signers::{LocalWallet, Signer},
    types::{transaction::eip2718::TypedTransaction, Address, Bytes, U256},
};
use mm_token_utils::{
    abi::{IUniswapV2PairAbigen, MemeTokenAbigen, UniswapV2FactoryAbigen, UniswapV2Router02Abigen},
    constants::{UNISWAP2_ROUTERS, WRAPPED_NATIVE_TOKENS},
    env::get_env,
    utils::{to_legacy_tx, to_signed_tx},
};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::constants::Env;

#[derive(Debug, Clone)]
pub struct Uniswap2Service {
    env: Env,
    http_provider: Arc<Provider<Http>>,
    gas_price: Arc<RwLock<U256>>,
    uniswapv2_router_address: Address,
    weth_address: Address,
    trading_slippage: f32,
    sell_tax: f32,
    buy_tax: f32,
    deployer_private_key: String,
}

impl Uniswap2Service {
    pub fn new(env: Env, gas_price: Arc<RwLock<U256>>, http_provider: Arc<Provider<Http>>) -> Self {
        let Some(uniswapv2_router_address) = UNISWAP2_ROUTERS.get(&env.listen_network) else {
            panic!("UNISWAP2_ROUTERS not found in {:?}", env.listen_network);
        };

        let Some(weth) = WRAPPED_NATIVE_TOKENS.get(&env.listen_network) else {
            panic!(
                "WRAPPED_NATIVE_TOKENS not found in {:?}",
                env.listen_network
            );
        };

        let deployer_private_key = get_env("DEPLOYER_PRIVATE_KEY", None).parse().unwrap();
        let trading_slippage: f32 = get_env("TRADING_SLIPPAGE", None).parse().unwrap_or(0.0);
        let sell_tax: f32 = get_env("TOKEN_SELL_TAX", None).parse().unwrap_or(0.0);
        let buy_tax: f32 = get_env("TOKEN_BUY_TAX", None).parse().unwrap_or(0.0);

        Self {
            env,
            http_provider,
            gas_price,
            uniswapv2_router_address: *uniswapv2_router_address,
            weth_address: weth.address,
            trading_slippage,
            sell_tax,
            buy_tax,
            deployer_private_key,
        }
    }

    pub async fn approve_token(
        &self,
        wallet_address: Address,
        nonce: Option<U256>,
        token_address: Address,
    ) -> anyhow::Result<TypedTransaction> {
        let nonce = nonce.unwrap_or(
            self.http_provider
                .get_transaction_count(wallet_address, None)
                .await?,
        );
        let token = IUniswapV2PairAbigen::new(token_address, self.http_provider.clone());
        let gas_price = *self.gas_price.read().await;

        let mut tx: TypedTransaction = token.approve(self.uniswapv2_router_address, U256::MAX).tx;
        tx.set_chain_id(self.env.chain_id);
        tx.set_from(wallet_address);
        tx.set_nonce(nonce);
        tx.set_gas(U256::from(500_000));
        tx.set_gas_price(gas_price);

        let approve_tx = to_legacy_tx(tx);

        Ok(approve_tx)
    }

    pub async fn buy_token(
        &self,
        mm_token_weth_pair_address: &Address,
        wallet_address: &Address,
        nonce: Option<U256>,
        buy_amount: U256,
        is_apply_slippage: bool,
    ) -> anyhow::Result<TypedTransaction> {
        let gas_price = *self.gas_price.read().await;
        let deadline = U256::from(Utc::now().timestamp()) + U256::from(60);

        let uniswapv2_router =
            UniswapV2Router02Abigen::new(self.uniswapv2_router_address, self.http_provider.clone());

        let amount_out_min = if is_apply_slippage {
            let total_slippage = self.trading_slippage + self.buy_tax;
            self.get_amount_out_min(
                *mm_token_weth_pair_address,
                true,
                buy_amount,
                total_slippage,
            )
            .await?
        } else {
            U256::one()
        };

        let nonce = nonce.unwrap_or(
            self.http_provider
                .get_transaction_count(*wallet_address, None)
                .await?,
        );

        let mut buy_tx = uniswapv2_router
            .swap_exact_eth_for_tokens_supporting_fee_on_transfer_tokens(
                amount_out_min,
                vec![self.weth_address, self.env.token_address],
                *wallet_address,
                deadline,
            )
            .tx;

        buy_tx.set_chain_id(self.env.chain_id);
        buy_tx.set_from(*wallet_address);
        buy_tx.set_nonce(nonce);
        buy_tx.set_gas(U256::from(500_000)); // fixed gas
        buy_tx.set_gas_price(gas_price);
        buy_tx.set_value(buy_amount);
        let buy_tx = to_legacy_tx(buy_tx);

        Ok(buy_tx)
    }

    pub async fn sell_token(
        &self,
        mm_token_weth_pair_address: &Address,
        wallet_address: &Address,
        nonce: Option<U256>,
        sell_amount: U256,
        is_apply_slippage: bool,
    ) -> anyhow::Result<TypedTransaction> {
        let gas_price = *self.gas_price.read().await;
        let deadline = U256::from(Utc::now().timestamp()) + U256::from(60);

        let uniswapv2_router =
            UniswapV2Router02Abigen::new(self.uniswapv2_router_address, self.http_provider.clone());

        let amount_out_min = if is_apply_slippage {
            let total_slippage = self.trading_slippage + self.sell_tax;
            self.get_amount_out_min(
                *mm_token_weth_pair_address,
                false,
                sell_amount,
                total_slippage,
            )
            .await?
        } else {
            U256::one()
        };

        let nonce = nonce.unwrap_or(
            self.http_provider
                .get_transaction_count(*wallet_address, None)
                .await?,
        );

        let mut sell_tx = uniswapv2_router
            .swap_exact_tokens_for_eth_supporting_fee_on_transfer_tokens(
                sell_amount,
                amount_out_min,
                vec![self.env.token_address, self.weth_address],
                *wallet_address,
                deadline,
            )
            .tx;

        sell_tx.set_chain_id(self.env.chain_id);
        sell_tx.set_from(*wallet_address);
        sell_tx.set_nonce(nonce);
        sell_tx.set_gas(U256::from(500_000)); // fixed gas
        sell_tx.set_gas_price(gas_price);
        let sell_tx = to_legacy_tx(sell_tx);

        Ok(sell_tx)
    }

    pub async fn compute_pair_address(
        &self,
        first_token: &Address,
        second_token: &Address,
    ) -> anyhow::Result<(Address, bool)> {
        let uniswapv2_router =
            UniswapV2Router02Abigen::new(self.uniswapv2_router_address, self.http_provider.clone());
        let factory_address: Address = uniswapv2_router.factory().call().await?;
        let uniswapv2_factory =
            UniswapV2FactoryAbigen::new(factory_address, self.http_provider.clone());
        let pair_address: Address = uniswapv2_factory
            .get_pair(*first_token, *second_token)
            .await?;

        if pair_address == Address::zero() {
            return Err(anyhow::anyhow!(
                "Pair address not found for the given tokens"
            ));
        }

        let uniswapv2_pair = IUniswapV2PairAbigen::new(pair_address, self.http_provider.clone());
        let token0_address = uniswapv2_pair.token_0().call().await?;

        // the second boolean is whether first_token is the token0 or not
        Ok((pair_address, *first_token == token0_address))
    }

    pub async fn get_all_pair_addresses(
        &self,
        first_token: &Address,
        second_token: &Address,
    ) -> anyhow::Result<Vec<Address>> {
        let (pair_address, _) = self.compute_pair_address(first_token, second_token).await?;

        Ok(vec![pair_address])
    }

    pub async fn get_amount_out_min(
        &self,
        mm_token_weth_pair_address: Address,
        is_buy: bool, // false for sell
        amount_in: U256,
        total_slippage: f32,
    ) -> anyhow::Result<U256> {
        let uniswapv2_pair =
            IUniswapV2PairAbigen::new(mm_token_weth_pair_address, self.http_provider.clone());
        let token0_address = uniswapv2_pair.token_0().call().await?;
        let is_mm_token0 = self.env.token_address == token0_address;

        let (reserve0, reserve1, _): (u128, u128, u32) =
            uniswapv2_pair.get_reserves().call().await?;
        let (mm_token_reserve, weth_reserve) = if is_mm_token0 {
            (reserve0, reserve1)
        } else {
            (reserve1, reserve0)
        };

        let uniswapv2_router =
            UniswapV2Router02Abigen::new(self.uniswapv2_router_address, self.http_provider.clone());

        let amount_out: U256 = if is_buy {
            uniswapv2_router
                .get_amount_out(amount_in, weth_reserve.into(), mm_token_reserve.into())
                .call()
                .await?
        } else {
            uniswapv2_router
                .get_amount_out(amount_in, mm_token_reserve.into(), weth_reserve.into())
                .call()
                .await?
        };

        let total_slippage_u256 = U256::from((total_slippage * 1000_f32).trunc() as u32);

        let amount_out_min = amount_out - amount_out * total_slippage_u256 / U256::from(100_000);

        Ok(amount_out_min)
    }

    pub async fn get_token_native_price(&self) -> anyhow::Result<(f64, u128, u128)> {
        let (pair, is_token0) = self
            .compute_pair_address(&self.env.token_address, &self.weth_address)
            .await?;

        let mm_token_weth_pair_address = pair;
        let is_mm_token0 = is_token0;

        let uniswapv2_pair =
            IUniswapV2PairAbigen::new(mm_token_weth_pair_address, self.http_provider.clone());
        let (reserve0, reserve1, _): (u128, u128, u32) =
            uniswapv2_pair.get_reserves().call().await?;
        let (mm_token_reserve, weth_reserve) = if is_mm_token0 {
            (reserve0, reserve1)
        } else {
            (reserve1, reserve0)
        };

        Ok(
            // token price
            (
                (BigDecimal::from(weth_reserve) / BigDecimal::from(mm_token_reserve))
                    .round(18)
                    .to_string()
                    .parse::<f64>()?,
                mm_token_reserve,
                weth_reserve,
            ),
        )
    }

    pub fn get_router_address(&self) -> anyhow::Result<Address> {
        Ok(self.uniswapv2_router_address)
    }

    pub async fn get_active_trading_tx(&self) -> anyhow::Result<Bytes> {
        let deployer_wallet = self
            .deployer_private_key
            .parse::<LocalWallet>()
            .unwrap()
            .with_chain_id(self.env.chain_id.as_u64());

        let token_contract =
            MemeTokenAbigen::new(self.env.token_address, self.http_provider.clone());
        let mut active_trading_tx: TypedTransaction = token_contract.activate_trading().tx;
        let nonce = self
            .http_provider
            .get_transaction_count(deployer_wallet.address(), None)
            .await?;

        let gas_price = *self.gas_price.read().await;
        // buff gas 5%
        let fixed_gas_price = gas_price * U256::from(105) / U256::from(100);

        active_trading_tx.set_chain_id(self.env.chain_id);
        active_trading_tx.set_from(deployer_wallet.address());
        active_trading_tx.set_nonce(nonce);
        active_trading_tx.set_gas(U256::from(500_000)); // fixed gas
        active_trading_tx.set_gas_price(fixed_gas_price);
        let active_trading_tx = to_legacy_tx(active_trading_tx);
        let signed_active_trading_tx = to_signed_tx(&deployer_wallet, &active_trading_tx).await?;

        Ok(signed_active_trading_tx)
    }
}
