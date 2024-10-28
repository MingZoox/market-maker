use crate::{
    constants::Env, core::MessageTransportService, routers::RouterService, utils::format_bmk,
};
use anyhow::anyhow;
use ethers::{
    middleware::SignerMiddleware,
    providers::{Http, Middleware, Provider},
    signers::{LocalWallet, Signer, WalletError},
    types::{
        transaction::eip2718::TypedTransaction, Address, TransactionReceipt, TransactionRequest,
        U256, U64,
    },
    utils::{format_ether, format_units, parse_ether},
};
use futures::future::join_all;
use mm_token_utils::{
    abi::{DisperseAbigen, IUniswapV2PairAbigen, MemeTokenAbigen},
    constants::WRAPPED_NATIVE_TOKENS,
    env::get_env,
    utils::{load_mnemonic_wallet, to_legacy_tx, to_signed_tx},
};
use provider_utils::constants::DESERIALIZATION_ERROR_MSG;
use rand::Rng;
use regex::Regex;
use std::{
    str::FromStr,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::{sync::RwLock, task};

#[derive(Debug, Clone)]
pub struct WalletService {
    env: Env,
    http_provider: Arc<Provider<Http>>,
    token_address: Address,
    weth_address: Address,
}

impl WalletService {
    pub fn new(env: Env, http_provider: Arc<Provider<Http>>) -> Self {
        let Some(weth) = WRAPPED_NATIVE_TOKENS.get(&env.listen_network) else {
            panic!(
                "WRAPPED_NATIVE_TOKENS not found in {:?}",
                env.listen_network
            );
        };
        Self {
            env,
            http_provider,
            token_address: Address::from_str(&get_env("TOKEN_ADDRESS", None)).unwrap(),
            weth_address: weth.address,
        }
    }

    /// Approve max token to another address
    pub async fn approve_max_to_seller(
        &self,
        approve_to_address: &Address,
        seller_wallet_index_from: u32,
        seller_wallet_index_to: u32,
    ) -> anyhow::Result<()> {
        let mut futures = Vec::new();
        for index in seller_wallet_index_from..(seller_wallet_index_to + 1) {
            let wallet_service_clone = self.clone();
            let approve_to_address_clone = *approve_to_address;
            let migrate_eth_future = task::spawn(async move {
                let wallet = wallet_service_clone.load_seller_wallets(index).unwrap();
                log::info!(
                    "wallet index {:?} address {:?} loaded",
                    index,
                    wallet.address()
                );

                let signer =
                    SignerMiddleware::new(wallet_service_clone.http_provider.clone(), wallet);
                let uniswapv2_pair =
                    IUniswapV2PairAbigen::new(wallet_service_clone.token_address, Arc::new(signer));
                let tx = uniswapv2_pair.approve(approve_to_address_clone, U256::MAX);
                match tx.send().await {
                    Ok(pending_tx) => {
                        log::info!("pending_tx {:?}", pending_tx.tx_hash());
                    }
                    Err(err) => {
                        log::error!("Failed to send transaction for index {}: {:?}", index, err);
                    }
                };
            });

            futures.push(migrate_eth_future);
        }
        join_all(futures).await;

        Ok(())
    }

    /// Check wallets' token and eth balance
    /// Allowance should be greater than or equal to balance
    pub async fn check_buyer_balance(&self) -> anyhow::Result<()> {
        let token_contract =
            MemeTokenAbigen::new(self.env.token_address, self.http_provider.clone());
        let decimals: u8 = token_contract.decimals().call().await?;
        let mut error_wallets = Vec::new();
        let mut total_token_balance = U256::zero();
        let mut total_eth_balance = U256::zero();

        let router_address = RouterService::new(
            self.env.clone(),
            Arc::new(RwLock::new(U256::zero())),
            self.http_provider.clone(),
        )
        .get_router_address()?;
        log::info!(
            "checking wallets' balances, uniswapv2_router={:?}",
            router_address
        );
        let buyer_wallets_count: u32 = get_env("BUYER_WALLETS_COUNT", None).parse().unwrap();
        for index in 0..buyer_wallets_count {
            let wallet = self.load_buyer_wallets(index)?;
            let wallet_address = wallet.address();

            let balance_of = token_contract.balance_of(wallet_address);
            let allowance = token_contract.allowance(wallet_address, router_address);
            let (balance, allowance, eth_balance) = tokio::join!(
                balance_of.call(),
                allowance.call(),
                self.http_provider.get_balance(wallet_address, None)
            );
            let balance: U256 = balance?;
            let allowance = allowance?;
            let eth_balance = eth_balance?;

            log::info!(
                "wallet_index {:?}, address {:?}, token_balance {:?}, eth_balance {:?}, allowance {:?}",
                index,
                wallet.address(),
                format_bmk(&format_units(balance, decimals as usize).unwrap(), 3)?,
                format_ether(eth_balance),
                allowance,
            );

            total_token_balance += balance;
            total_eth_balance += eth_balance;

            if allowance < balance {
                log::warn!(
                    "invalid allowance, allowance {:?}, balance {:?}",
                    allowance,
                    balance
                );
                error_wallets.push(wallet_address);
            }
            if eth_balance < parse_ether("0.005").unwrap() {
                log::warn!("invalid eth balance, eth_balance {:?}", eth_balance,);
                error_wallets.push(wallet_address);
            }
        }

        log::info!(
            "TOTAL REPORT: total_token_balance {:?}, total_eth_balance {:?}",
            format_bmk(
                &format_units(total_token_balance, decimals as usize).unwrap(),
                3
            )?,
            format_ether(total_eth_balance)
        );
        if !error_wallets.is_empty() {
            log::warn!("Please check these wallets {:?}", error_wallets);
        }

        Ok(())
    }

    /// Migrate all buyer wallets' token to seller wallets
    pub async fn migrate_token_buyer_to_seller(&self) -> anyhow::Result<()> {
        let message_transport_service = MessageTransportService::new();
        let mut index = 0;
        let buyer_wallets_count: u32 = get_env("BUYER_WALLETS_COUNT", None).parse().unwrap();
        while index < buyer_wallets_count {
            let wallet = self.load_buyer_wallets(index)?;
            let migration_wallet = self.load_seller_wallets(index)?;
            let (from_wallet_address, to_wallet_address) =
                (wallet.address(), migration_wallet.address());
            log::info!(
                "migrate token index {:?} from_wallet buyer {:?} to_wallet seller {:?} processing",
                index,
                from_wallet_address,
                to_wallet_address
            );
            let message = format!(
                "Migrate token \nIndex {:?} from_wallet buyer {:?} to_wallet seller {:?} processing",
                index, from_wallet_address, to_wallet_address
            );
            message_transport_service.send_message(message).await?;

            let signer = SignerMiddleware::new(self.http_provider.clone(), wallet);
            let token = IUniswapV2PairAbigen::new(self.env.token_address, Arc::new(signer.clone()));
            let token_balance: U256 = token.balance_of(from_wallet_address).call().await?;
            if token_balance > U256::zero() {
                let tx_receipt: Option<TransactionReceipt> = token
                    .transfer(to_wallet_address, token_balance)
                    .send()
                    .await?
                    .await?;
                log::info!(
                    "sent token tx_hash={:?}",
                    tx_receipt.map(|x| x.transaction_hash)
                );
            } else {
                log::warn!("skip because of zero token balance");
            }

            index += 1;
        }

        Ok(())
    }

    /// Migrate all buyer wallets' eth to seller wallets
    pub async fn migrate_eth_buyer_to_seller(&self) -> anyhow::Result<()> {
        // let message_transport_service = MessageTransportService::new();
        let mut index = 0;
        let buyer_wallets_count: u32 = get_env("BUYER_WALLETS_COUNT", None).parse().unwrap();
        let mut futures = Vec::new();
        while index < buyer_wallets_count {
            let wallet_service_clone = self.clone();
            let migrate_eth_future = task::spawn(async move {
                let wallet = wallet_service_clone.load_buyer_wallets(index).unwrap();
                let migration_wallet = wallet_service_clone.load_seller_wallets(index).unwrap();
                let (from_wallet_address, to_wallet_address) =
                    (wallet.address(), migration_wallet.address());
                log::info!(
                    "migrate eth index {:?} from_wallet buyer {:?} to_wallet seller {:?} processing",
                    index,
                    from_wallet_address,
                    to_wallet_address
                );

                let signer =
                    SignerMiddleware::new(wallet_service_clone.http_provider.clone(), wallet);
                // handle transfer surplus balance from buyer to seller wallet
                'migrate_surplus_balance: loop {
                    if let Err(err) = WalletService::send_entire_eth_balance(
                        &signer,
                        from_wallet_address,
                        to_wallet_address,
                    )
                    .await
                    {
                        log::warn!("rerun because resend overshot failed err={:?}", err);
                        continue 'migrate_surplus_balance;
                    } else {
                        break 'migrate_surplus_balance;
                    }
                }
            });

            futures.push(migrate_eth_future);

            index += 1;
        }
        join_all(futures).await;

        Ok(())
    }

    /// Migrate all buyer wallets' token to seller wallets
    pub async fn migrate_token_to_seller_by_index(
        &self,
        wallet_index: u32,
        buy_nonce: U256,
        fetched_gas_price: U256,
    ) -> anyhow::Result<()> {
        let wallet = self.load_buyer_wallets(wallet_index)?;
        let migration_wallet = self.load_seller_wallets(wallet_index)?;
        let (from_wallet_address, to_wallet_address) =
            (wallet.clone().address(), migration_wallet.address());

        let signer = SignerMiddleware::new(self.http_provider.clone(), wallet.clone());
        let token = MemeTokenAbigen::new(self.env.token_address, self.http_provider.clone());

        let mut token_balance;

        let start_time = Instant::now();
        let timeout_duration = Duration::from_secs(300); // Timeout duration of 30 seconds

        loop {
            token_balance = token.balance_of(from_wallet_address).call().await?;
            if token_balance > U256::zero() {
                break;
            }

            if start_time.elapsed() >= timeout_duration {
                log::info!(
                    "Timeout get token balance reached at wallet index {:#?}!",
                    wallet_index
                );
                return Ok(());
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        let mut transfer_tx: TypedTransaction = token.transfer(to_wallet_address, token_balance).tx;

        let migrate_nonce = buy_nonce + 1;

        transfer_tx.set_chain_id(self.env.chain_id);
        transfer_tx.set_from(wallet.address());
        transfer_tx.set_nonce(migrate_nonce);
        transfer_tx.set_gas(U256::from(500_000)); // fixed gas
        transfer_tx.set_gas_price(fetched_gas_price);
        let transfer_tx = to_legacy_tx(transfer_tx);
        let signed_transfer_tx = to_signed_tx(&wallet, &transfer_tx).await?;

        let pending_tx = self
            .http_provider
            .send_raw_transaction(signed_transfer_tx.clone())
            .await;

        match pending_tx {
            Ok(_pending_tx) => {
                let tx_receipt = _pending_tx
                    .await?
                    .ok_or(anyhow!("Cannot find tx_receipt"))?;
                if tx_receipt.status == Some(U64::zero()) {
                    log::warn!("Transaction {} failed", tx_receipt.transaction_hash);
                } else {
                    log::info!(
                        "Success tx migrate token wallet_index {:#?} from {:#?} to {:#?}: {:#?}",
                        wallet_index,
                        from_wallet_address,
                        to_wallet_address,
                        tx_receipt.transaction_hash
                    );

                    // handle transfer surplus balance from buyer to seller wallet
                    'migrate_surplus_balance: loop {
                        if let Err(err) = WalletService::send_entire_eth_balance(
                            &signer,
                            from_wallet_address,
                            to_wallet_address,
                        )
                        .await
                        {
                            log::warn!("rerun because resend overshot failed err={:?}", err);
                            continue 'migrate_surplus_balance;
                        } else {
                            break 'migrate_surplus_balance;
                        }
                    }
                };
            }
            Err(err) => {
                log::warn!(
                    "Sent migrate token fail from wallet index {:#?}: {:?}",
                    wallet_index,
                    err
                );
            }
        }

        Ok(())
    }

    /// Send entire eth balance to another address
    pub async fn send_entire_eth_balance(
        signer: &SignerMiddleware<Arc<Provider<Http>>, LocalWallet>,
        from_address: Address,
        to_address: Address,
    ) -> anyhow::Result<()> {
        let balance = signer.get_balance(from_address, None).await?;
        if balance == U256::zero() {
            log::warn!("skip because of zero eth balance");
            return Ok(());
        }
        let gas_price = signer.get_gas_price().await? * U256::from(101) / U256::from(100);
        let gas_limit = 21_000;
        let gas_cost_wei = gas_price * gas_limit;
        if gas_cost_wei >= balance {
            log::warn!("skip because of approximately zero eth balance");
            return Ok(());
        }
        let mut total_wei_to_send = balance - gas_cost_wei;
        let tx = TransactionRequest::new()
            .to(to_address)
            .value(total_wei_to_send)
            .gas(gas_limit)
            .gas_price(gas_price);
        let pending_tx = signer.send_transaction(tx, None).await;
        let Err(err) = pending_tx else {
            let tx_receipt = pending_tx?
                .await?
                .ok_or(anyhow!("Cannot find tx_receipt"))?;
            if tx_receipt.status == Some(U64::zero()) {
                log::warn!(
                    "sent eth fail from {:#?} to {:#?}, tx_hash={:#?}",
                    from_address,
                    to_address,
                    tx_receipt.transaction_hash
                );
            } else {
                log::info!(
                    "sent eth success from {:#?} to {:#?}, tx_hash={:#?}",
                    from_address,
                    to_address,
                    tx_receipt.transaction_hash
                )
            };

            return Ok(());
        };

        let re: Regex = Regex::new(r"overshot (?P<overshot>\d+)").unwrap();
        let err_str = err.to_string();
        let Some(captures) = re.captures(&err_str) else {
            return Err(err.into());
        };
        let overshot = U256::from_dec_str(&captures["overshot"])?;
        log::warn!("resend overshot={:?}", overshot);
        total_wei_to_send -= overshot;
        let tx_receipt = signer
            .send_transaction(
                TransactionRequest::new()
                    .to(to_address)
                    .value(total_wei_to_send)
                    .gas(gas_limit)
                    .gas_price(gas_price),
                None,
            )
            .await?
            .await?;
        let Some(tx_receipt) = tx_receipt else {
            return Err(anyhow::anyhow!("overshot failed"));
        };
        log::info!("sent eth tx_hash={:?}", tx_receipt.transaction_hash);

        Ok(())
    }

    /// disperse eth to another address
    pub async fn disperse_eth(
        &self,
        disperse_eth_private_key: &str,
        disperse_eth_mnemonic: &str,
        disperse_eth_amount: U256,
        disperse_router: Address,
        wallet_index_from: u32,
        wallet_index_to: u32,
    ) -> anyhow::Result<()> {
        if wallet_index_from > wallet_index_to {
            log::error!("invalid index");
            return Ok(());
        }
        let wallet_size = wallet_index_to - wallet_index_from + 1;

        let total_disperse_value = disperse_eth_amount * U256::from(wallet_size);
        let disperse_wallet = disperse_eth_private_key
            .parse::<LocalWallet>()
            .unwrap()
            .with_chain_id(self.env.clone().chain_id.as_u64());
        let disperse_wallet_balance = self
            .http_provider
            .get_balance(disperse_wallet.address(), None)
            .await?;
        if disperse_wallet_balance < total_disperse_value {
            log::error!("disperse_wallet balance not enough for disperse");
            return Ok(());
        }

        let mut recipients = Vec::new();
        let transfer_values: Vec<U256> = vec![disperse_eth_amount; wallet_size as usize];
        for index in wallet_index_from..wallet_index_to + 1 {
            let wallet = load_mnemonic_wallet(disperse_eth_mnemonic, index)?;
            recipients.push(wallet.address());
        }

        let signer = Arc::new(SignerMiddleware::new(
            self.http_provider.clone(),
            disperse_wallet,
        ));

        let disperse = DisperseAbigen::new(disperse_router, signer);
        let disperse_fn = disperse
            .disperse_ether(recipients, transfer_values)
            .value(total_disperse_value);
        let disperse_tx = disperse_fn.send().await?;

        log::info!(
            "Disperse ETH for buyer wallets at tx: {:#?}",
            disperse_tx.tx_hash()
        );
        Ok(())
    }

    /// disperse token to another address
    #[allow(clippy::too_many_arguments)]
    pub async fn disperse_tokens(
        &self,
        disperse_router: Address,
        disperse_token_private_key: &str,
        disperse_token_mnemonic: &str,
        wallet_index_from: u32,
        wallet_index_to: u32,
        disperse_token_amount_min: u128,
        disperse_token_amount_max: u128,
    ) -> anyhow::Result<()> {
        // vec: wallet address and token amount
        let mut target_wallets_address = Vec::<Address>::new();
        let mut target_wallets_token_amount = Vec::<u128>::new();

        for index in wallet_index_from..(wallet_index_to + 1) {
            let wallet = self.load_mnemonic_wallet(disperse_token_mnemonic, index)?;
            target_wallets_address.push(wallet.address());
            let random_token_amount =
                rand::thread_rng().gen_range(disperse_token_amount_min..=disperse_token_amount_max);
            target_wallets_token_amount.push(random_token_amount);
        }

        let disperse_wallet = disperse_token_private_key
            .parse::<LocalWallet>()
            .unwrap()
            .with_chain_id(self.env.clone().chain_id.as_u64());

        let signer = Arc::new(SignerMiddleware::new(
            self.http_provider.clone(),
            disperse_wallet.clone(),
        ));
        let token = IUniswapV2PairAbigen::new(self.token_address, signer.clone());

        let total_token_amount_disperse: u128 = target_wallets_token_amount.iter().sum();

        let balance_of = token.balance_of(disperse_wallet.address());
        let allowance = token.allowance(disperse_wallet.address(), disperse_router);
        let token_decimals = token.decimals();
        let (token_balance, token_decimals, allowance) =
            tokio::join!(balance_of.call(), token_decimals.call(), allowance.call());
        let token_balance = token_balance?;
        let token_decimals = token_decimals?;
        let disperse_wallet_allowance = allowance?;

        let total_token_amount_disperse_with_decimals =
            U256::from(total_token_amount_disperse) * U256::exp10(token_decimals as usize);

        if token_balance < total_token_amount_disperse_with_decimals {
            log::warn!("Token balance lower than total_token_amount_disperse");
            return Ok(());
        }

        if disperse_wallet_allowance < total_token_amount_disperse_with_decimals {
            log::info!("approving token for disperse_router {:#?}", disperse_router);
            match token.approve(disperse_router, U256::MAX).send().await {
                Ok(result) => {
                    log::info!(
                        "approved token tx hash: {:#?}",
                        result.await?.unwrap().transaction_hash
                    );
                }
                Err(err) => {
                    log::error!("Error in approving token: {:#?}", err);
                    return Ok(());
                }
            };
        }

        let disperse = DisperseAbigen::new(disperse_router, signer);
        log::info!("target_wallets_address: {:#?}", target_wallets_address);
        log::info!(
            "target_wallets_token_amount: {:#?}",
            target_wallets_token_amount
        );
        let target_wallets_token_amount = target_wallets_token_amount
            .iter()
            .map(|&x| U256::from(x) * U256::exp10(token_decimals as usize))
            .collect();
        let disperse_fn = disperse.disperse_token(
            self.token_address,
            target_wallets_address,
            target_wallets_token_amount,
        );
        let disperse_tx = disperse_fn.send().await?;

        log::info!(
            "Disperse token for target wallets at tx: {:#?}",
            disperse_tx.tx_hash()
        );

        Ok(())
    }

    pub async fn dump_all(
        &self,
        gas_price: Arc<RwLock<U256>>,
        dump_interval_min: u32,
        dump_interval_max: u32,
    ) -> anyhow::Result<()> {
        let router_service =
            RouterService::new(self.env.clone(), gas_price, self.http_provider.clone());
        let token_contract =
            MemeTokenAbigen::new(self.env.token_address, self.http_provider.clone());

        let router_address = router_service.get_router_address()?;

        // update flex for any mnemonic later
        let buyer_wallets_count: u32 = get_env("BUYER_WALLETS_COUNT", None).parse().unwrap();

        let mut index: u32 = 0;
        loop {
            if index >= buyer_wallets_count {
                break;
            }
            let buyer_wallet = self.load_buyer_wallets(index).unwrap();

            let balance_of = token_contract.balance_of(buyer_wallet.address());
            let allowance = token_contract.allowance(buyer_wallet.address(), router_address);
            let (token_balance, allowance) = tokio::join!(balance_of.call(), allowance.call());
            let token_balance = token_balance?;
            let allowance = allowance?;

            if token_balance.is_zero() {
                log::info!(
                    "Buyer wallet {:#?} don't have token, skip",
                    buyer_wallet.address()
                );
                index += 1;
                continue;
            }

            let signer = SignerMiddleware::new(self.http_provider.clone(), buyer_wallet.clone());
            if allowance < token_balance {
                log::info!("approving token wallet {:#?}", buyer_wallet.address());

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

            log::info!(
                "Selling all tokens in buyer wallet {:#?}",
                buyer_wallet.address()
            );

            let (pair_address, _) = router_service
                .get_pair_address(&self.env.token_address, &self.weth_address, false)
                .await?;

            let signed_sell_tx = router_service
                .construct_sell_token_tx(&buyer_wallet, None, token_balance, &pair_address, true)
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

            let dump_interval = rand::thread_rng().gen_range(dump_interval_min..=dump_interval_max);
            let sleep_duration = Duration::from_secs(dump_interval as u64);
            log::info!(
                "token sold tx_hash={:?}, dump_interval={:?}s",
                sell_tx_receipt.map(|x| x.transaction_hash),
                dump_interval
            );
            tokio::time::sleep(sleep_duration).await;

            index += 1;
        }

        Ok(())
    }

    pub fn load_buyer_wallets(&self, index: u32) -> Result<LocalWallet, WalletError> {
        let buyer_mnemonic: String = get_env("BUYER_MNEMONIC", None);
        self.load_mnemonic_wallet(&buyer_mnemonic, index)
    }

    pub fn load_seller_wallets(&self, index: u32) -> Result<LocalWallet, WalletError> {
        let seller_mnemonic: String = get_env("SELLER_MNEMONIC", None);
        self.load_mnemonic_wallet(&seller_mnemonic, index)
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
