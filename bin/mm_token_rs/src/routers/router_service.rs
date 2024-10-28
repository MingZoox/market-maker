use super::{Uniswap2Service, Uniswap3Service};
use crate::constants::Env;
use ethers::{
    providers::{Http, Provider},
    signers::{LocalWallet, Signer},
    types::{transaction::eip2718::TypedTransaction, Address, Bytes, U256},
};
use mm_token_utils::{constants::ERouter, env::get_env, utils::to_signed_tx};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone)]
pub struct RouterService {
    pub active_router: ERouter,
    uniswap2_service: Uniswap2Service,
    uniswap3_service: Uniswap3Service,
}

impl RouterService {
    pub fn new(env: Env, gas_price: Arc<RwLock<U256>>, http_provider: Arc<Provider<Http>>) -> Self {
        let uniswap2_service =
            Uniswap2Service::new(env.clone(), gas_price.clone(), http_provider.clone());
        let uniswap3_service =
            Uniswap3Service::new(env.clone(), gas_price.clone(), http_provider.clone());
        let active_router: ERouter = get_env("ACTIVE_ROUTER", None).parse().unwrap();

        Self {
            active_router,
            uniswap2_service,
            uniswap3_service,
        }
    }

    // buy
    pub async fn construct_buy_token_tx(
        &self,
        wallet: &LocalWallet,
        nonce: Option<U256>,
        buy_amount: U256,
        pair_address: &Address,
        is_apply_slippage: bool,
    ) -> anyhow::Result<Bytes> {
        let buy_tx = match self.active_router {
            ERouter::Uniswap2Routers => {
                self.uniswap2_service
                    .buy_token(
                        pair_address,
                        &wallet.address(),
                        nonce,
                        buy_amount,
                        is_apply_slippage,
                    )
                    .await?
            }
            ERouter::Uniswap3Routers => {
                self.uniswap3_service
                    .buy_token(
                        pair_address,
                        &wallet.address(),
                        nonce,
                        buy_amount,
                        is_apply_slippage,
                    )
                    .await?
            }
            ERouter::UniversalRouters => TypedTransaction::default(),
        };
        let signed_buy_tx = to_signed_tx(wallet, &buy_tx).await?;

        Ok(signed_buy_tx)
    }

    // sell
    pub async fn construct_sell_token_tx(
        &self,
        wallet: &LocalWallet,
        nonce: Option<U256>,
        sell_amount: U256,
        pair_address: &Address,
        is_apply_slippage: bool,
    ) -> anyhow::Result<Bytes> {
        let sell_tx = match self.active_router {
            ERouter::Uniswap2Routers => {
                self.uniswap2_service
                    .sell_token(
                        pair_address,
                        &wallet.address(),
                        nonce,
                        sell_amount,
                        is_apply_slippage,
                    )
                    .await?
            }
            ERouter::Uniswap3Routers => {
                self.uniswap3_service
                    .sell_token(
                        pair_address,
                        &wallet.address(),
                        nonce,
                        sell_amount,
                        is_apply_slippage,
                    )
                    .await?
            }
            ERouter::UniversalRouters => TypedTransaction::default(),
        };
        let signed_sell_tx = to_signed_tx(wallet, &sell_tx).await?;

        Ok(signed_sell_tx)
    }

    pub async fn get_token_native_price(
        &self,
        active_router: ERouter,
        pair_address: Address,
    ) -> anyhow::Result<f64> {
        match active_router {
            ERouter::Uniswap2Routers => {
                let (price, _, _) = self.uniswap2_service.get_token_native_price().await?;
                Ok(price)
            }
            ERouter::Uniswap3Routers => Ok(self
                .uniswap3_service
                .get_token_native_price(pair_address)
                .await?),
            // TODO: update universal ver later
            ERouter::UniversalRouters => {
                let (price, _, _) = self.uniswap2_service.get_token_native_price().await?;
                Ok(price)
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn get_amount_out(
        &self,
        active_router: ERouter,
        pool_address: &Address,
        is_buy: bool,
        token_in: Option<&Address>,
        token_out: Option<&Address>,
        amount_in: U256,
        total_slippage: f32,
    ) -> anyhow::Result<U256> {
        let amount_out: U256 = match active_router {
            ERouter::Uniswap2Routers => {
                self.uniswap2_service
                    .get_amount_out_min(*pool_address, is_buy, amount_in, total_slippage)
                    .await?
            }
            ERouter::Uniswap3Routers => {
                self.uniswap3_service
                    .get_amount_out_by_slippage(
                        pool_address,
                        token_in.unwrap(),
                        token_out.unwrap(),
                        amount_in,
                        total_slippage,
                    )
                    .await?
            }
            // TODO: update universal ver later
            ERouter::UniversalRouters => {
                self.uniswap2_service
                    .get_amount_out_min(*pool_address, is_buy, amount_in, total_slippage)
                    .await?
            }
        };

        Ok(amount_out)
    }

    pub async fn get_pair_address(
        &self,
        first_token: &Address,
        second_token: &Address,
        is_buy: bool,
    ) -> anyhow::Result<(Address, bool)> {
        let pair_address = match self.active_router {
            ERouter::Uniswap2Routers => {
                self.uniswap2_service
                    .compute_pair_address(first_token, second_token)
                    .await?
            }
            ERouter::Uniswap3Routers => {
                self.uniswap3_service
                    .compute_pair_address(first_token, second_token, is_buy, None)
                    .await?
            }
            // TODP: update later
            ERouter::UniversalRouters => {
                self.uniswap2_service
                    .compute_pair_address(first_token, second_token)
                    .await?
            }
        };

        Ok(pair_address)
    }

    pub async fn get_pair_address_by_router(
        &self,
        first_token: &Address,
        second_token: &Address,
        is_buy: bool,
        fee_tier_v3: Option<u32>,
        router: ERouter,
    ) -> anyhow::Result<(Address, bool)> {
        let pair_address = match router {
            ERouter::Uniswap2Routers => {
                self.uniswap2_service
                    .compute_pair_address(first_token, second_token)
                    .await?
            }
            ERouter::Uniswap3Routers => {
                self.uniswap3_service
                    .compute_pair_address(first_token, second_token, is_buy, fee_tier_v3)
                    .await?
            }
            // TODP: update later
            ERouter::UniversalRouters => {
                self.uniswap2_service
                    .compute_pair_address(first_token, second_token)
                    .await?
            }
        };

        Ok(pair_address)
    }

    pub async fn get_all_pair_addresses(
        &self,
        first_token: &Address,
        second_token: &Address,
    ) -> anyhow::Result<Vec<Address>> {
        let sell_receivers = match self.active_router {
            ERouter::Uniswap2Routers => {
                self.uniswap2_service
                    .get_all_pair_addresses(first_token, second_token)
                    .await?
            }
            ERouter::Uniswap3Routers => {
                self.uniswap3_service
                    .get_all_pair_addresses(first_token, second_token)
                    .await?
            }
            ERouter::UniversalRouters => todo!(),
        };

        Ok(sell_receivers)
    }

    pub fn get_router_address(&self) -> anyhow::Result<Address> {
        let address = match self.active_router {
            ERouter::Uniswap2Routers => self.uniswap2_service.get_router_address()?,
            ERouter::Uniswap3Routers => self.uniswap3_service.get_router_address()?,
            ERouter::UniversalRouters => todo!(),
        };

        Ok(address)
    }

    pub async fn get_active_trading_tx(&self) -> anyhow::Result<Bytes> {
        let future = match self.active_router {
            ERouter::Uniswap2Routers => self.uniswap2_service.get_active_trading_tx().await?,
            ERouter::Uniswap3Routers => self.uniswap3_service.get_active_trading_tx().await?,
            ERouter::UniversalRouters => todo!(),
        };

        Ok(future)
    }
}
