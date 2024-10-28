use std::{sync::Arc, time::Duration};

use crate::{
    types::*,
    utils::{compute_system_wallets, get_mm_config},
};
use ethers::{
    providers::{Http, Middleware, Provider},
    types::{Address, U256},
    utils::{format_units, parse_ether},
};
use mm_token_utils::{
    abi::{IUniswapV2PairAbigen, MemeTokenAbigen},
    constants::{
        Erc20Details, AVABOT_ROUTERS, UNISWAP2_ROUTERS, WRAPPED_NATIVE_TOKENS, ZERO_ADDRESS,
    },
    env::get_env,
};
use provider_utils::http_providers::HttpProviders;

use crate::constants::Env;

use super::LaunchingProcessService;

#[derive(Debug, Clone)]
pub struct ApiService {
    pub env: Env,
    pub http_provider: Arc<Provider<Http>>,
    pub uniswapv2_router_address: Address,
    pub avabot_router_address: Address,
    pub weth: Erc20Details,
    buyer_mnemonic: String,
    buyer_surplus_balance: U256,
    buyer_wallets_count: u32,
    auto_buyer_mnemonic: String,
    auto_buyer_wallets_count: u32,
    seller_mnemonic: String,
    seller_wallets_count: u32,
    auto_sell_min_percent: u32,
    auto_sell_max_percent: u32,
}

#[warn(unused_variables)]
impl ApiService {
    pub fn new() -> Self {
        let env = Env::new();
        let Ok(http_provider) = HttpProviders::get_first_provider(&env.listen_network, false)
        else {
            panic!("[ApiService] http_provider not found");
        };
        let Some(avabot_router_address) = AVABOT_ROUTERS.get(&env.listen_network) else {
            panic!("AVABOT_ROUTERS not found in {:?}", env.listen_network);
        };
        let Some(uniswapv2_router_address) = UNISWAP2_ROUTERS.get(&env.listen_network) else {
            panic!("UNISWAP2_ROUTERS not found in {:?}", env.listen_network);
        };
        let Some(weth) = WRAPPED_NATIVE_TOKENS.get(&env.listen_network) else {
            panic!(
                "WRAPPED_NATIVE_TOKENS not found in {:?}",
                env.listen_network
            );
        };
        Self {
            env,
            http_provider: Arc::new(http_provider),
            uniswapv2_router_address: *uniswapv2_router_address,
            avabot_router_address: *avabot_router_address,
            weth: weth.clone(),
            buyer_mnemonic: get_env("BUYER_MNEMONIC", None),
            buyer_surplus_balance: parse_ether(get_env("BUYER_SURPLUS_BALANCE", None)).unwrap(),
            buyer_wallets_count: get_env("BUYER_WALLETS_COUNT", None).parse().unwrap(),
            auto_buyer_mnemonic: get_env("AUTO_BUYER_MNEMONIC", None),
            auto_buyer_wallets_count: get_env("AUTO_BUYER_WALLETS_COUNT", None).parse().unwrap(),
            seller_mnemonic: get_env("SELLER_MNEMONIC", None),
            seller_wallets_count: get_env("SELLER_WALLETS_COUNT", None).parse().unwrap(),
            auto_sell_min_percent: get_env("AUTO_SELL_MIN_PERCENT", None).parse().unwrap(),
            auto_sell_max_percent: get_env("AUTO_SELL_MAX_PERCENT", None).parse().unwrap(),
        }
    }

    pub async fn get_network_status(&self) -> NetworkStatus {
        let network_str = get_env("LISTEN_NETWORK", None);
        let current_block_number = self.http_provider.get_block_number().await.unwrap();
        let Some(weth) = WRAPPED_NATIVE_TOKENS.get(&self.env.listen_network) else {
            panic!(
                "WRAPPED_NATIVE_TOKENS not found in {:?}",
                self.env.listen_network
            );
        };

        let token_info_call =
            MemeTokenAbigen::new(self.env.token_address, self.http_provider.clone());
        let token_symbol: String = token_info_call.symbol().call().await.unwrap();
        let token_name: String = token_info_call.name().call().await.unwrap();
        let token_decimals: u8 = token_info_call.decimals().call().await.unwrap();
        let token_total_supply: U256 = token_info_call.total_supply().call().await.unwrap();

        NetworkStatus {
            network: NetworkStatusNetworkInfo {
                name: network_str,
                chain_id: self.env.chain_id.as_u64(),
                block_number: current_block_number.as_u64(),
            },
            token: NetworkStatusTokenInfo {
                address: self.env.token_address,
                is_deployed: true,
                symbol: token_symbol,
                name: token_name,
                decimals: token_decimals,
                total_supply: (token_total_supply / U256::exp10(token_decimals as usize)).as_u128(),
                token_template: TokenTemplate::BaseMemeTokenV1,
                router_contract: self.uniswapv2_router_address,
                pair_contract: *ZERO_ADDRESS, // TODO: pair address ?
                weth: weth.address,
            },
            router: NetworkStatusRouterInfo {
                avabot: self.avabot_router_address,
            },
        }
    }

    pub async fn get_deployment_checklist(&self) -> DeploymentChecklist {
        let buyer_system_wallets = compute_system_wallets(
            &self.buyer_mnemonic,
            self.buyer_wallets_count,
            &self.env.token_address,
            self.http_provider.clone(),
        )
        .await
        .unwrap();

        let mut buyer_balance_status = true;
        let mut buyer_balance: String = "".to_string();
        for (wallet_address, wallet) in buyer_system_wallets {
            let Ok(wallet_context) = wallet.try_write() else {
                continue;
            };

            if wallet_context.eth_balance <= self.buyer_surplus_balance {
                buyer_balance_status = false;
                buyer_balance += &(wallet_address.to_string() + ", ");
            }
        }

        let whitelist_added_info = "".to_string();

        let mut buyer_balance_info = "".to_string();
        if !buyer_balance_status {
            buyer_balance_info =
                "Address ".to_owned() + &buyer_balance + "don't have sufficient balance";
        }

        let seller_system_wallets = compute_system_wallets(
            &self.seller_mnemonic,
            self.seller_wallets_count,
            &self.env.token_address,
            self.http_provider.clone(),
        )
        .await
        .unwrap();
        let uniswapv2_pair =
            IUniswapV2PairAbigen::new(self.env.token_address, self.http_provider.clone());
        let mut seller_approval_status = true;
        let mut seller_approval: String = "".to_string();

        for (wallet_address, _wallet) in seller_system_wallets {
            let allowance = uniswapv2_pair
                .allowance(wallet_address, self.uniswapv2_router_address)
                .call()
                .await
                .unwrap();

            if allowance == U256::zero() {
                seller_approval_status = false;
                seller_approval += &(wallet_address.to_string() + ", ");
            }
        }
        let mut seller_approval_info = "".to_string();
        if !seller_approval_status {
            seller_approval_info =
                "Address ".to_owned() + &seller_approval + "have not been approved yet";
        }

        // TODO: update later
        let (reserve0, _reserve1, _): (u128, u128, u32) =
            uniswapv2_pair.get_reserves().call().await.unwrap();

        let liquidity_added_status: bool = reserve0 == 0;

        DeploymentChecklist {
            token_deployed: TokenDeployed { status: true },
            whitelist_added: WhitelistAdded {
                status: false,
                info: whitelist_added_info,
            },
            buyer_balance: BuyerBalance {
                status: buyer_balance_status,
                info: buyer_balance_info,
            },
            seller_approval: SellerApproval {
                status: seller_approval_status,
                info: seller_approval_info,
            },
            liquidity_added: LiquidityAdded {
                status: liquidity_added_status,
                info: "".to_string(),
            },
        }
    }

    pub async fn get_deployer(&self) -> Deployer {
        let Some(weth) = WRAPPED_NATIVE_TOKENS.get(&self.env.listen_network) else {
            panic!(
                "WRAPPED_NATIVE_TOKENS not found in {:?}",
                self.env.listen_network
            );
        };
        let token_info_call =
            MemeTokenAbigen::new(self.env.token_address, self.http_provider.clone());
        let deployer_address = token_info_call.owner().call().await.unwrap();
        let deployer_balance = self
            .http_provider
            .get_balance(deployer_address, None)
            .await
            .unwrap();
        Deployer {
            address: deployer_address,
            balance: format_units(deployer_balance, weth.decimals as usize)
                .expect("Failed to format units"),
        }
    }

    pub async fn get_buyers(&self) -> Buyers {
        let buyer_system_wallets = compute_system_wallets(
            &self.buyer_mnemonic,
            self.buyer_wallets_count,
            &self.env.token_address,
            self.http_provider.clone(),
        )
        .await
        .unwrap();

        let token_info_call =
            MemeTokenAbigen::new(self.env.token_address, self.http_provider.clone());
        let token_decimals: u8 = token_info_call.decimals().call().await.unwrap();

        let mut total_balance = U256::from(0);
        let mut total_token_balance = U256::from(0);
        let mut list_wallets_info = Vec::<BuyersWalletInfo>::new();

        for (wallet_address, wallet) in buyer_system_wallets.iter() {
            let Ok(wallet_context) = wallet.try_write() else {
                continue;
            };

            let wallet_info = BuyersWalletInfo {
                path: "m/44'/60'/0'/0/".to_string() + &wallet_context.index.to_string(),
                address: *wallet_address,
                balance: format_units(wallet_context.eth_balance, self.weth.decimals as usize)
                    .expect("Failed to format units"),
                token_balance: format_units(
                    wallet_context.token_balance,
                    (token_decimals + 6) as usize,
                )
                .expect("Failed to format units")
                    + "M",
            };
            total_balance += wallet_context.eth_balance;
            total_token_balance += wallet_context.token_balance;
            list_wallets_info.push(wallet_info);
        }

        Buyers {
            settings: BuyersSettings {
                surplus_amount: get_env("BUYER_SURPLUS_BALANCE", None),
            },
            status: BuyersStatus {
                total_balance: format_units(total_balance, self.weth.decimals as usize)
                    .expect("Failed to format units"),
                total_token_balance: format_units(
                    total_token_balance,
                    (token_decimals + 9) as usize,
                )
                .expect("Failed to format units")
                    + "B",
            },
            list: list_wallets_info,
        }
    }

    pub async fn get_auto_buyers(&self) -> Buyers {
        let buyer_system_wallets = compute_system_wallets(
            &self.auto_buyer_mnemonic,
            self.auto_buyer_wallets_count,
            &self.env.token_address,
            self.http_provider.clone(),
        )
        .await
        .unwrap();

        let token_info_call =
            MemeTokenAbigen::new(self.env.token_address, self.http_provider.clone());
        let token_decimals: u8 = token_info_call.decimals().call().await.unwrap();

        let mut total_balance = U256::from(0);
        let mut total_token_balance = U256::from(0);
        let mut list_wallets_info = Vec::<BuyersWalletInfo>::new();

        for (wallet_address, wallet) in buyer_system_wallets.iter() {
            let Ok(wallet_context) = wallet.try_write() else {
                continue;
            };

            let wallet_info = BuyersWalletInfo {
                path: "m/44'/60'/0'/0/".to_string() + &wallet_context.index.to_string(),
                address: *wallet_address,
                balance: format_units(wallet_context.eth_balance, self.weth.decimals as usize)
                    .expect("Failed to format units"),
                token_balance: format_units(
                    wallet_context.token_balance,
                    (token_decimals + 6) as usize,
                )
                .expect("Failed to format units")
                    + "M",
            };
            total_balance += wallet_context.eth_balance;
            total_token_balance += wallet_context.token_balance;
            list_wallets_info.push(wallet_info);
        }

        Buyers {
            settings: BuyersSettings {
                surplus_amount: get_env("AUTO_BUYER_SURPLUS_BALANCE", None),
            },
            status: BuyersStatus {
                total_balance: format_units(total_balance, self.weth.decimals as usize)
                    .expect("Failed to format units"),
                total_token_balance: format_units(
                    total_token_balance,
                    (token_decimals + 6) as usize,
                )
                .expect("Failed to format units")
                    + "M",
            },
            list: list_wallets_info,
        }
    }

    pub async fn get_sellers(&self) -> Sellers {
        let Some(weth) = WRAPPED_NATIVE_TOKENS.get(&self.env.listen_network) else {
            panic!(
                "WRAPPED_NATIVE_TOKENS not found in {:?}",
                self.env.listen_network
            );
        };

        let token_contract =
            MemeTokenAbigen::new(self.env.token_address, self.http_provider.clone());
        let token_decimals: u8 = token_contract.decimals().call().await.unwrap();

        let seller_system_wallets = compute_system_wallets(
            &self.seller_mnemonic,
            self.seller_wallets_count,
            &self.env.token_address,
            self.http_provider.clone(),
        )
        .await
        .unwrap();

        let mut total_balance = U256::from(0);
        let mut total_token_balance = U256::from(0);
        let mut list_wallets_info = Vec::<SellersWalletInfo>::new();

        for (wallet_address, wallet) in seller_system_wallets {
            let Ok(wallet_context) = wallet.try_write() else {
                continue;
            };

            let allowance_uniswapv2_router = token_contract
                .allowance(wallet_address, self.uniswapv2_router_address)
                .call()
                .await
                .unwrap();
            let allowance_ava_router = token_contract
                .allowance(wallet_address, self.avabot_router_address)
                .call()
                .await
                .unwrap();

            let wallet_info = SellersWalletInfo {
                path: "m/44'/60'/0'/0/".to_string() + &wallet_context.index.to_string(),
                address: wallet_address,
                balance: format_units(wallet_context.eth_balance, weth.decimals as usize)
                    .expect("Failed to format units"),
                token_balance: format_units(
                    wallet_context.token_balance,
                    (token_decimals + 6) as usize,
                )
                .expect("Failed to format units")
                    + "M",
                approvals: ApprovalsSellers {
                    token_router: format_units(
                        allowance_uniswapv2_router,
                        (weth.decimals + 12) as usize,
                    )
                    .expect("Failed to format units")
                        + "T",
                    ava_router: format_units(allowance_ava_router, (weth.decimals + 12) as usize)
                        .expect("Failed to format units")
                        + "T",
                },
            };
            total_balance += wallet_context.eth_balance;
            total_token_balance += wallet_context.token_balance;
            list_wallets_info.push(wallet_info);
        }

        Sellers {
            settings: SellersSettings {
                volume_threshold: get_env("BUYER_SURPLUS_BALANCE", None),
                min_percent: self.auto_sell_min_percent,
                max_percent: self.auto_sell_max_percent,
            },
            status: SellersStatus {
                total_balance: format_units(total_balance, weth.decimals as usize)
                    .expect("Failed to format units"),
                total_token_balance: format_units(
                    total_token_balance,
                    (token_decimals + 6) as usize,
                )
                .expect("Failed to format units")
                    + "M",
            },
            list: list_wallets_info,
        }
    }

    pub async fn get_market_makers(&self) -> MarketMakers {
        let mut mm_group_list = Vec::<MarketMakersGroup>::new();
        let mm_config: MmConfig = get_mm_config();
        let mut total_balance = U256::from(0);
        let token_contract =
            MemeTokenAbigen::new(self.env.token_address, self.http_provider.clone());
        // MM configs
        for (mm_index, group_setting) in mm_config.groups.iter().enumerate() {
            let mm_group_wallets = compute_system_wallets(
                &group_setting.mnemonic,
                group_setting
                    .max_wallets_count
                    .unwrap_or(mm_config.default_settings.max_wallets_count),
                &self.env.token_address,
                self.http_provider.clone(),
            )
            .await
            .unwrap();

            let mut mm_wallet_info_list = Vec::<MarketMakersWalletInfo>::new();
            for (wallet_address, wallet) in mm_group_wallets {
                let Ok(wallet_context) = wallet.try_write() else {
                    continue;
                };
                let allowance_uniswapv2_router = token_contract
                    .allowance(wallet_address, self.uniswapv2_router_address)
                    .call()
                    .await
                    .unwrap();
                let allowance_ava_router = token_contract
                    .allowance(wallet_address, self.avabot_router_address)
                    .call()
                    .await
                    .unwrap();

                let wallet_info = MarketMakersWalletInfo {
                    path: "m/44'/60'/0'/0/".to_string() + &wallet_context.index.to_string(),
                    address: wallet_address,
                    balance: format_units(wallet_context.eth_balance, self.weth.decimals as usize)
                        .expect("Failed to format units"),
                    token_balance: format_units(
                        wallet_context.token_balance,
                        (self.weth.decimals + 6) as usize,
                    )
                    .expect("Failed to format units")
                        + "M",
                    approvals: ApprovalsMarketMakers {
                        token_router: format_units(
                            allowance_uniswapv2_router,
                            (self.weth.decimals + 12) as usize,
                        )
                        .expect("Failed to format units")
                            + "T",
                        ava_router: format_units(
                            allowance_ava_router,
                            (self.weth.decimals + 12) as usize,
                        )
                        .expect("Failed to format units")
                            + "T",
                    },
                };
                total_balance += wallet_context.eth_balance;
                mm_wallet_info_list.push(wallet_info);
            }

            let mut group_setting = group_setting.clone();
            group_setting.mnemonic = "".to_string();
            let mm_group = MarketMakersGroup {
                index: mm_index as u8,
                settings: group_setting,
                mm_wallet_info: mm_wallet_info_list,
            };

            mm_group_list.push(mm_group);
        }

        MarketMakers {
            default_settings: mm_config.default_settings,
            status: MarketMakersStatus {
                total_balance: format_units(total_balance, self.weth.decimals as usize)
                    .expect("Failed to format units"),
            },
            list: mm_group_list,
        }
    }

    pub async fn launch_process(&self) -> LaunchStatus {
        let mut status = LaunchStatus {
            active_trading: StepStatus::Pending,
            buyers_bot_launch: StepStatus::Pending,
            migrate_tokens_to_seller: StepStatus::Pending,
            start_auto_sell: StepStatus::Pending,
            market_making_launch: StepStatus::Pending,
        };
        let http_provider = Arc::new(
            HttpProviders::get_healthy_provider(&self.env.listen_network, false)
                .await
                .unwrap(),
        );
        let launching_process_service =
            LaunchingProcessService::new(self.env.clone(), http_provider);

        match launching_process_service.active_trading_and_buy().await {
            Ok(_) => status.active_trading = StepStatus::Activated,
            Err(error) => {
                status.active_trading = StepStatus::Error(error.to_string());
                return status;
            }
        }

        tokio::time::sleep(Duration::from_secs(10)).await;

        let (auto_sell_result, market_making_result) = tokio::join!(
            launching_process_service.start_auto_sell(),
            launching_process_service.start_market_making()
        );

        if auto_sell_result.is_ok() {
            status.start_auto_sell = StepStatus::Activated;
        } else {
            status.start_auto_sell = StepStatus::Error(auto_sell_result.err().unwrap().to_string());
            return status;
        }

        if market_making_result.is_ok() {
            status.market_making_launch = StepStatus::Activated;
        } else {
            status.market_making_launch =
                StepStatus::Error(market_making_result.err().unwrap().to_string());
            return status;
        }

        status
    }
}

impl Default for ApiService {
    fn default() -> Self {
        Self::new()
    }
}
