use bigdecimal::BigDecimal;
use ethers::{
    types::U256,
    utils::{format_ether, format_units, parse_ether},
};
use mm_token_rs::{
    core::ApiService,
    types::{Buyers, Deployer, DeploymentChecklist, LaunchStatus, MarketMakers, NetworkStatus},
};
use mm_token_utils::abi::MemeTokenAbigen;
#[derive(Debug, Clone)]
pub struct CommandService {
    // env: Env,
    api_service: ApiService,
}

impl Default for CommandService {
    fn default() -> Self {
        Self::new()
    }
}

impl CommandService {
    pub fn new() -> Self {
        let api_service = ApiService::new();
        Self { api_service }
    }

    // APIs
    pub async fn get_network_status(&self) -> NetworkStatus {
        self.api_service.get_network_status().await
    }

    pub async fn get_deployment_checklist(&self) -> DeploymentChecklist {
        self.api_service.get_deployment_checklist().await
    }

    pub async fn get_deployer(&self) -> Deployer {
        self.api_service.get_deployer().await
    }

    pub async fn get_buyers(&self) -> Buyers {
        self.api_service.get_buyers().await
    }

    pub async fn get_auto_buyers(&self) -> String {
        let buyers_info = self.api_service.get_auto_buyers().await;
        log::info!("buyers_info: {:#?}", buyers_info);

        let auto_buyers_total_eth = buyers_info.status.total_balance;
        let auto_buyers_total_token = buyers_info.status.total_token_balance;

        let auto_buyers_info_summary_content = self
            .process_summary_info(&auto_buyers_total_eth, &auto_buyers_total_token)
            .await
            .unwrap();

        let title = "💶 Auto Buy Summary 💶\n".to_string();

        title + &auto_buyers_info_summary_content
    }

    pub async fn get_sellers(&self) -> String {
        let sellers_info = self.api_service.get_sellers().await;
        log::info!("sellers_info: {:#?}", sellers_info);

        let sellers_total_eth = sellers_info.status.total_balance;
        let sellers_total_token = sellers_info.status.total_token_balance;

        let sellers_info_summary_content = self
            .process_summary_info(&sellers_total_eth, &sellers_total_token)
            .await
            .unwrap();

        let title = "📊 Auto Sell Summary 📊\n".to_string();

        title + &sellers_info_summary_content
    }

    pub async fn get_market_makers(&self) -> MarketMakers {
        self.api_service.get_market_makers().await
    }

    pub async fn launch_process(&self) -> LaunchStatus {
        self.api_service.launch_process().await
    }

    // launch process commands
    pub async fn launch_buy_bot(&self) -> anyhow::Result<()> {
        Ok(())
    }

    pub async fn launch_sell_bot(&self) -> anyhow::Result<()> {
        Ok(())
    }

    // common func
    pub async fn process_summary_info(
        &self,
        total_eth_str: &str,
        total_token_str: &str,
    ) -> anyhow::Result<String> {
        // - ETH price: $3200. Balance: 100 ETH ~ $320,000
        // - Token price: 0.0001 ETH ~ $0.32. Balance: 1M token ~ 100 ETH ~ $320,000
        // - Pool: 1000 ETH + 10M token. Liquidity: $6,400,000
        // - FDV: $10,000,000 (lấy giá token nhân với total supply)

        let mut res_message = "\n".to_string();
        // URL of the CoinGecko API to get Ethereum price
        let url = "https://api.coingecko.com/api/v3/simple/price?ids=ethereum&vs_currencies=usd";
        let response = reqwest::get(url).await?;

        // ETH info
        let mut eth_price: f64 = 0.0;
        if response.status().is_success() {
            let body = response.text().await?;
            let json: serde_json::Value = serde_json::from_str(&body)?;

            // Extract the price of Ethereum (ETH) from the JSON
            if let Some(price) = json["ethereum"]["usd"].as_f64() {
                eth_price = price;
                log::info!("Current Ethereum (ETH) price: ${}", price);
            } else {
                log::warn!("Price data not found in the response.");
            }
        } else {
            log::warn!(
                "Failed to get Ethereum price. Status code: {}",
                response.status()
            );
        }

        let total_balance_dollar = total_eth_str.parse::<f64>().unwrap() * eth_price;
        let eth_info = format!(
            "- ETH price: ${:#?}. Balance: {:.4} ETH ~ ${:.2}\n\n",
            eth_price,
            total_eth_str.parse::<f64>().unwrap(),
            total_balance_dollar
        );
        res_message.push_str(&eth_info);

        // Token Info
        let (mm_token_reserve, weth_reserve, token_total_supply, token_decimals, token_symbol) =
            self.get_reverse_and_total_supply().await?;
        log::info!(
            "mm_token_reserve: {:#?}, weth_reserve: {:#?}",
            mm_token_reserve,
            weth_reserve
        );
        let token_price_eth = (BigDecimal::from(weth_reserve) / BigDecimal::from(mm_token_reserve))
            .round(18)
            .to_string()
            .parse::<f64>()?;
        log::info!("token_price_eth: {:#?}", token_price_eth);
        let token_price_dollar = token_price_eth * eth_price;

        let total_token =
            total_token_str.replace('M', "").parse::<f64>().unwrap() * f64::powi(10.0, 6);

        let total_token_price_eth = token_price_eth * total_token;
        let total_token_price_dollar = total_token_price_eth * eth_price;

        let token_info = format!(
            "- Token price: {:#?} ETH ~ ${:#?}. Balance: {:.6}M {:#?} ~ {:.4} ETH ~ ${:.2}\n\n",
            token_price_eth,
            token_price_dollar,
            total_token_str.replace('M', "").parse::<f64>().unwrap(),
            token_symbol,
            total_token_price_eth,
            total_token_price_dollar
        );
        res_message.push_str(&token_info);

        // Pool info
        let weth_decimals = self.api_service.weth.decimals;
        let weth_pool_reverse =
            format_units(weth_reserve, weth_decimals as usize).expect("Failed to format units");
        let token_pool_reverse = format_units(mm_token_reserve, token_decimals as usize + 6)
            .expect("Failed to format units");

        let liquidity = format_ether(
            parse_ether(token_price_dollar)? * U256::from(mm_token_reserve)
                / U256::exp10(token_decimals as usize)
                + parse_ether(eth_price)? * U256::from(weth_reserve)
                    / U256::exp10(weth_decimals as usize),
        );

        let pool_info = format!(
            "- Pool: {:.4} ETH + {:.6}M {:#?}. Liquidity: ${:.6}\n\n",
            weth_pool_reverse.parse::<f64>().unwrap(),
            token_pool_reverse.parse::<f64>().unwrap(),
            token_symbol,
            liquidity.parse::<f64>().unwrap()
        );
        res_message.push_str(&pool_info);

        // FDV info
        let fdv = format_ether(
            parse_ether(token_price_dollar)? * token_total_supply
                / U256::exp10(token_decimals as usize),
        )
        .parse::<f64>()
        .unwrap();
        let fdv_info = format!("- FDV: ${:.2}", fdv);
        res_message.push_str(&fdv_info);

        Ok(res_message)
    }

    async fn get_reverse_and_total_supply(&self) -> anyhow::Result<(u128, u128, U256, u8, String)> {
        let api_service = self.api_service.clone();

        let token_info_call = MemeTokenAbigen::new(
            api_service.env.token_address,
            api_service.http_provider.clone(),
        );
        let token_symbol: String = token_info_call.symbol().call().await.unwrap();
        // let token_name: String = token_info_call.name().call().await.unwrap();
        let token_decimals: u8 = token_info_call.decimals().call().await.unwrap();
        let token_total_supply: U256 = token_info_call.total_supply().call().await.unwrap();

        // TODO: reverse ?
        Ok((
            0_u128,
            0_u128,
            token_total_supply,
            token_decimals,
            token_symbol,
        ))
    }
}
