use anyhow::anyhow;
use bigdecimal::BigDecimal;
use ethers::{
    providers::{Http, Middleware, Provider},
    signers::{LocalWallet, Signer},
    types::{transaction::eip2718::TypedTransaction, Address, Bytes, U256},
    utils::parse_ether,
};
use mm_token_utils::{
    abi::{
        ExactInputSingleParams, MemeTokenAbigen, QuoteExactInputSingleParams, QuoterV2Abigen,
        UniswapV3FactoryAbigen, UniswapV3PoolAbigen, UniswapV3Router02Abigen,
    },
    constants::{UNISWAP3_QUOTER_V2, UNISWAP3_ROUTERS, WRAPPED_NATIVE_TOKENS, ZERO_ADDRESS},
    env::get_env,
    utils::{to_legacy_tx, to_signed_tx},
};
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::constants::Env;

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum UniswapV3FeeTier {
    Tier500 = 500,
    Tier3000 = 3000,
    Tier10000 = 10000,
}

#[allow(clippy::from_over_into)]
impl Into<u32> for UniswapV3FeeTier {
    fn into(self) -> u32 {
        match self {
            UniswapV3FeeTier::Tier500 => 500,
            UniswapV3FeeTier::Tier3000 => 3000,
            UniswapV3FeeTier::Tier10000 => 10000,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Uniswap3Service {
    env: Env,
    http_provider: Arc<Provider<Http>>,
    gas_price: Arc<RwLock<U256>>,
    weth_address: Address,
    uniswap_v3_router_address: Address,
    uniswap_v3_quoter_v2_address: Address,
    trading_slippage: f32,
    sell_tax: f32,
    buy_tax: f32,
    deployer_private_key: String,
}

impl Uniswap3Service {
    pub fn new(env: Env, gas_price: Arc<RwLock<U256>>, http_provider: Arc<Provider<Http>>) -> Self {
        let Some(uniswap_v3_router_address) = UNISWAP3_ROUTERS.get(&env.listen_network) else {
            panic!("UNISWAP3_ROUTERS not found in {:?}", env.listen_network);
        };

        let Some(uniswap_v3_quoter_v2_address) = UNISWAP3_QUOTER_V2.get(&env.listen_network) else {
            panic!("UNISWAP3_QUOTER_V2 not found in {:?}", env.listen_network);
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
            weth_address: weth.address,
            uniswap_v3_router_address: *uniswap_v3_router_address,
            uniswap_v3_quoter_v2_address: *uniswap_v3_quoter_v2_address,
            trading_slippage,
            sell_tax,
            buy_tax,
            deployer_private_key,
        }
    }

    pub async fn buy_token(
        &self,
        pool_address: &Address,
        recipient: &Address,
        recipient_nonce: Option<U256>,
        amount_in: U256,
        is_apply_slippage: bool,
    ) -> anyhow::Result<TypedTransaction> {
        let uniswapv3_pool = UniswapV3PoolAbigen::new(*pool_address, self.http_provider.clone());

        let liquidity: u128 = uniswapv3_pool.liquidity().call().await?;
        if liquidity == 0 {
            return Err(anyhow!(
                "[Uniswap3Service.buy_token] Pool without liquidity {:?}",
                pool_address
            ));
        }

        let pool_fee: u32 = uniswapv3_pool.fee().call().await?;

        let gas_price = *self.gas_price.read().await;
        let uniswapv3_router = UniswapV3Router02Abigen::new(
            self.uniswap_v3_router_address,
            self.http_provider.clone(),
        );

        let amount_out_minimum = if is_apply_slippage {
            let total_slippage = self.trading_slippage + self.buy_tax;
            self.get_amount_out_by_slippage(
                pool_address,
                &self.weth_address,
                &self.env.token_address,
                amount_in,
                total_slippage,
            )
            .await?
        } else {
            U256::zero()
        };

        let recipient_nonce = recipient_nonce.unwrap_or(
            self.http_provider
                .get_transaction_count(*recipient, None)
                .await?,
        );

        let mut buy_tx: TypedTransaction = uniswapv3_router
            .exact_input_single(ExactInputSingleParams {
                token_in: self.weth_address,
                token_out: self.env.token_address,
                fee: pool_fee,
                recipient: *recipient,
                amount_in,
                amount_out_minimum,
                sqrt_price_limit_x96: U256::zero(),
            })
            .tx;
        buy_tx.set_chain_id(self.env.chain_id);
        buy_tx.set_from(*recipient);
        buy_tx.set_nonce(recipient_nonce);
        buy_tx.set_gas(U256::from(700_000)); // fixed gas
        buy_tx.set_gas_price(gas_price);

        let buy_tx = to_legacy_tx(buy_tx);

        Ok(buy_tx)
    }

    pub async fn sell_token(
        &self,
        pool_address: &Address,
        recipient: &Address,
        recipient_nonce: Option<U256>,
        amount_in: U256,
        is_apply_slippage: bool,
    ) -> anyhow::Result<TypedTransaction> {
        let uniswapv3_pool = UniswapV3PoolAbigen::new(*pool_address, self.http_provider.clone());

        let liquidity: u128 = uniswapv3_pool.liquidity().call().await?;
        if liquidity == 0 {
            return Err(anyhow!(
                "[Uniswap3Service.buy_token] Pool without liquidity {:?}",
                pool_address
            ));
        }

        let pool_fee: u32 = uniswapv3_pool.fee().call().await?;

        let gas_price = *self.gas_price.read().await;
        let uniswapv3_router = UniswapV3Router02Abigen::new(
            self.uniswap_v3_router_address,
            self.http_provider.clone(),
        );

        let total_slippage = self.trading_slippage + self.sell_tax;
        let amount_out_minimum = if is_apply_slippage {
            self.get_amount_out_by_slippage(
                pool_address,
                &self.env.token_address,
                &self.weth_address,
                amount_in,
                total_slippage,
            )
            .await?
        } else {
            U256::zero()
        };

        let recipient_nonce = recipient_nonce.unwrap_or(
            self.http_provider
                .get_transaction_count(*recipient, None)
                .await?,
        );

        let mut sell_tx: TypedTransaction = uniswapv3_router
            .exact_input_single(ExactInputSingleParams {
                token_in: self.env.token_address,
                token_out: self.weth_address,
                fee: pool_fee,
                recipient: *recipient,
                amount_in,
                amount_out_minimum,
                sqrt_price_limit_x96: U256::zero(),
            })
            .tx;

        sell_tx.set_chain_id(self.env.chain_id);
        sell_tx.set_from(*recipient);
        sell_tx.set_nonce(recipient_nonce);
        sell_tx.set_gas(U256::from(700_000)); // fixed gas
        sell_tx.set_gas_price(gas_price);

        let sell_tx = to_legacy_tx(sell_tx);
        Ok(sell_tx)
    }

    pub async fn get_amount_out_by_slippage(
        &self,
        pool_address: &Address,
        token_in: &Address,
        token_out: &Address,
        amount_in: U256,
        total_slippage: f32,
    ) -> anyhow::Result<U256> {
        let uniswapv3_pool = UniswapV3PoolAbigen::new(*pool_address, self.http_provider.clone());
        let pool_fee: u32 = uniswapv3_pool.fee().call().await?;

        let quoter_v2 = QuoterV2Abigen::new(
            self.uniswap_v3_quoter_v2_address,
            self.http_provider.clone(),
        );
        let (amount_out, _, _, _) = match quoter_v2
            .quote_exact_input_single(QuoteExactInputSingleParams {
                token_in: *token_in,
                token_out: *token_out,
                amount_in,
                fee: pool_fee,
                sqrt_price_limit_x96: U256::zero(),
            })
            .call()
            .await
        {
            Ok(result) => result,
            Err(err) => {
                let revert_data = err.to_string();
                log::warn!("[quote_exact_input_single] with error: {:?}", revert_data);
                return Ok(U256::zero());
            }
        };

        let total_slippage_u256 = U256::from((total_slippage * 1000_f32).trunc() as u32);

        let amount_out_min = amount_out - amount_out * total_slippage_u256 / U256::from(100_000);

        Ok(amount_out_min)
    }

    pub async fn compute_pair_address(
        &self,
        first_token: &Address,
        second_token: &Address,
        is_buy: bool,
        fee_tier_v3: Option<u32>,
    ) -> anyhow::Result<(Address, bool)> {
        let uniswapv3_router = UniswapV3Router02Abigen::new(
            self.uniswap_v3_router_address,
            self.http_provider.clone(),
        );
        let factory_address: Address = uniswapv3_router.factory().call().await?;

        let uniswapv3_factory =
            UniswapV3FactoryAbigen::new(factory_address, self.http_provider.clone());
        let mut max_amount_out = U256::zero();
        let mut max_pair_address = *ZERO_ADDRESS;
        let mut is_first_token_0 = false;

        if fee_tier_v3.is_some() {
            let pool_address: Address = uniswapv3_factory
                .get_pool(*first_token, *second_token, fee_tier_v3.unwrap())
                .await?;

            return Ok((pool_address, false));
        }

        for fee_tier in &[
            UniswapV3FeeTier::Tier500,
            UniswapV3FeeTier::Tier3000,
            UniswapV3FeeTier::Tier10000,
        ] {
            let pair_address: Address = uniswapv3_factory
                .get_pool(*first_token, *second_token, (*fee_tier).into())
                .await?;

            if pair_address.eq(&ZERO_ADDRESS) {
                continue;
            }

            let is_first_token_weth = *first_token == self.weth_address;

            let mut amount_out_min = U256::zero();
            if is_buy && is_first_token_weth {
                amount_out_min = self
                    .get_amount_out_by_slippage(
                        &pair_address,
                        first_token,
                        second_token,
                        U256::from(100), // simulate number
                        0.0,
                    )
                    .await?;
            };
            if is_buy && !is_first_token_weth {
                amount_out_min = self
                    .get_amount_out_by_slippage(
                        &pair_address,
                        second_token,
                        first_token,
                        U256::from(100), // simulate number
                        0.0,
                    )
                    .await?;
            };
            if !is_buy && is_first_token_weth {
                amount_out_min = self
                    .get_amount_out_by_slippage(
                        &pair_address,
                        second_token,
                        first_token,
                        U256::from(100), // simulate number
                        0.0,
                    )
                    .await?;
            };
            if !is_buy && !is_first_token_weth {
                amount_out_min = self
                    .get_amount_out_by_slippage(
                        &pair_address,
                        first_token,
                        second_token,
                        U256::from(100), // simulate number
                        0.0,
                    )
                    .await?;
            };

            if amount_out_min > max_amount_out {
                max_amount_out = amount_out_min;
                max_pair_address = pair_address;

                let uniswap_v3_pool =
                    UniswapV3PoolAbigen::new(pair_address, self.http_provider.clone());
                let token0_address: Address = uniswap_v3_pool.token_0().call().await?;
                is_first_token_0 = *first_token == token0_address;
            }
        }

        Ok((max_pair_address, is_first_token_0))
    }

    pub async fn get_all_pair_addresses(
        &self,
        first_token: &Address,
        second_token: &Address,
    ) -> anyhow::Result<Vec<Address>> {
        let uniswapv3_router = UniswapV3Router02Abigen::new(
            self.uniswap_v3_router_address,
            self.http_provider.clone(),
        );
        let factory_address: Address = uniswapv3_router.factory().call().await?;

        let uniswapv2_factory =
            UniswapV3FactoryAbigen::new(factory_address, self.http_provider.clone());

        let mut pair_addresses: Vec<Address> = Vec::new();

        for fee_tier in &[
            UniswapV3FeeTier::Tier500,
            UniswapV3FeeTier::Tier3000,
            UniswapV3FeeTier::Tier10000,
        ] {
            let pair_address: Address = uniswapv2_factory
                .get_pool(*first_token, *second_token, (*fee_tier).into())
                .await?;

            if pair_address.eq(&ZERO_ADDRESS) {
                continue;
            }

            pair_addresses.push(pair_address);
        }

        Ok(pair_addresses)
    }

    pub async fn get_token_native_price(&self, pool_address: Address) -> anyhow::Result<f64> {
        let uniswapv3_pool = UniswapV3PoolAbigen::new(pool_address, self.http_provider.clone());
        let (sqrt_price_x96, _, _, _, _, _, _): (U256, i32, u16, u16, u16, u8, bool) =
            uniswapv3_pool.slot_0().call().await?;
        let token0: Address = uniswapv3_pool.token_0().call().await?;

        let ten_pow_18 = BigDecimal::from_str(&parse_ether(1).unwrap().to_string())?; // reducing value to avoid `arithmetic operation overflow`

        let sqrt_price_x96 =
            BigDecimal::from_str(&sqrt_price_x96.to_string())? / ten_pow_18.clone();
        let sqrt_price_x96_pow2 = sqrt_price_x96.clone() * sqrt_price_x96.clone();

        let two_pow_192 = BigDecimal::from_str(&(U256::from(2).pow(U256::from(192))).to_string())?
            / (ten_pow_18.clone() * ten_pow_18.clone());

        let token0_token1_ratio = (sqrt_price_x96_pow2 / two_pow_192)
            .round(18)
            .to_string()
            .parse::<f64>()?;

        if token0.eq(&self.weth_address) {
            return Ok(1_f64 / token0_token1_ratio);
        }

        Ok(token0_token1_ratio)
    }

    pub fn get_router_address(&self) -> anyhow::Result<Address> {
        Ok(self.uniswap_v3_router_address)
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
