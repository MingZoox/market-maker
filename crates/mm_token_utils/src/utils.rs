use std::fmt::LowerHex;

use ::serde::{Deserialize, Serialize};
use bip39::{Language, Mnemonic, MnemonicType};
use ethers::{
    abi::{ethabi, ParamType, Token, Tokenizable},
    signers::{coins_bip39::English, LocalWallet, MnemonicBuilder, Signer, WalletError},
    types::{
        transaction::eip2718::TypedTransaction, Address, Bytes, TransactionRequest, H160, U256,
    },
    utils::keccak256,
};
use provider_utils::enums::ENetwork;

use crate::constants::{V2_SWAP_EXACT_IN, V2_SWAP_EXACT_OUT, V3_SWAP_EXACT_IN, V3_SWAP_EXACT_OUT};

pub fn compute_transaction_hash(raw_tx: &Bytes) -> String {
    format!("0x{}", hex::encode(keccak256(raw_tx)))
}

pub fn to_legacy_tx(tx: TypedTransaction) -> TypedTransaction {
    match tx {
        TypedTransaction::Eip1559(inner) => {
            let tx: TransactionRequest = inner.into();
            TypedTransaction::Legacy(tx)
        }
        other => other,
    }
}

pub async fn to_signed_tx(
    wallet: &LocalWallet,
    tx: &TypedTransaction,
) -> Result<Bytes, WalletError> {
    let signature = wallet.sign_transaction(tx).await?;
    let signed = tx.rlp_signed(&signature);
    Ok(signed)
}

pub fn load_mnemonic_wallet(mnemonic: &str, index: u32) -> Result<LocalWallet, WalletError> {
    let wallet = MnemonicBuilder::<English>::default()
        .phrase(mnemonic)
        .index(index)?
        .build()?;
    Ok(wallet)
}

/// Generate a random 12-words mnemonic phrase
pub fn random_mnemonic_phrase() -> String {
    let mnemonic = Mnemonic::new(MnemonicType::Words12, Language::English);
    mnemonic.into_phrase()
}

pub fn get_wallet_path_prefix(network: ENetwork) -> String {
    match network {
        ENetwork::EthSepolia | ENetwork::EthMainnet => "m/44'/60'/0'/0/".to_string(),
        ENetwork::FtmTestnet | ENetwork::FtmMainnet => "m/44'/214'/0'/0/".to_string(),
        _ => "m/0'/0'/0'/0/".to_string(),
    }
}

pub fn format_lower_hex(hash: &impl LowerHex) -> String {
    format!("{:#x}", hash)
}

pub fn universal_decode(command: u8, input: Vec<u8>) -> SwapUniversalRouterInfo {
    match command {
        V2_SWAP_EXACT_IN => decode_v2_swap_exact_in(input),
        V2_SWAP_EXACT_OUT => decode_v2_swap_exact_out(input),
        V3_SWAP_EXACT_IN => decode_v3_swap_exact_in(input),
        V3_SWAP_EXACT_OUT => decode_v3_swap_exact_out(input),
        _ => SwapUniversalRouterInfo::default(),
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct SwapUniversalRouterInfo {
    pub amount_in: U256,
    pub amount_out: U256,
    pub path: Vec<H160>,
}

pub fn decode_v2_swap_exact_in(input: Vec<u8>) -> SwapUniversalRouterInfo {
    // ABI for the function signature
    log::info!("From v2_swap_exact_in");
    let abi = vec![
        ParamType::Address,
        ParamType::Uint(256),
        ParamType::Uint(256),
        ParamType::Array(Box::new(ParamType::Address)),
        ParamType::Bool,
    ];

    let tokens: Vec<Token> = ethabi::decode(&abi, &input).unwrap();

    // let recipient = tokens[0].clone().into_address().unwrap();
    let amount_in = tokens[1].clone().into_uint().unwrap();
    // let amount_out_min = tokens[2].clone().into_uint().unwrap();
    let path: Vec<H160> = tokens[3]
        .clone()
        .into_array()
        .unwrap()
        .iter()
        .map(|token| token.clone().into_address().unwrap())
        .collect();
    // let payer_is_user = tokens[4].clone().into_bool().unwrap();

    SwapUniversalRouterInfo {
        amount_in,
        amount_out: U256::zero(),
        path,
    }
}

pub fn decode_v2_swap_exact_out(input: Vec<u8>) -> SwapUniversalRouterInfo {
    log::info!("From v2_swap_exact_out");
    // ABI for the function signature
    let abi = vec![
        ParamType::Address,
        ParamType::Uint(256),
        ParamType::Uint(256),
        ParamType::Array(Box::new(ParamType::Address)),
        ParamType::Bool,
    ];

    let tokens: Vec<Token> = ethabi::decode(&abi, &input).unwrap();

    // let recipient = tokens[0].clone().into_address().unwrap();
    let amount_out = tokens[1].clone().into_uint().unwrap();
    // let amount_in_max = tokens[2].clone().into_uint().unwrap();
    let path: Vec<H160> = tokens[3]
        .clone()
        .into_array()
        .unwrap()
        .iter()
        .map(|token| token.clone().into_address().unwrap())
        .collect();
    // let payer_is_user = tokens[4].clone().into_bool().unwrap();

    SwapUniversalRouterInfo {
        amount_in: U256::zero(),
        amount_out,
        path,
    }
}

pub fn decode_v3_swap_exact_in(input: Vec<u8>) -> SwapUniversalRouterInfo {
    log::info!("From v3_swap_exact_in");
    // ABI for the function signature
    let abi = vec![
        ParamType::Address,
        ParamType::Uint(256),
        ParamType::Uint(256),
        ParamType::Bytes,
        ParamType::Bool,
    ];

    let tokens: Vec<Token> = ethabi::decode(&abi, &input).unwrap();

    // let recipient = tokens[0].clone().into_address().unwrap();
    let amount_in = tokens[1].clone().into_uint().unwrap();
    // let amount_out_min = tokens[2].clone().into_uint().unwrap();
    let full_path = Bytes::from_token(tokens[3].clone()).unwrap().to_vec();
    let path: Vec<H160> = extract_path_from_v3(full_path, false);
    // let payer_is_user = tokens[4].clone().into_bool().unwrap();

    SwapUniversalRouterInfo {
        amount_in,
        amount_out: U256::zero(),
        path,
    }
}

pub fn decode_v3_swap_exact_out(input: Vec<u8>) -> SwapUniversalRouterInfo {
    log::info!("From v3_swap_exact_out");
    // ABI for the function signature
    let abi = vec![
        ParamType::Address,
        ParamType::Uint(256),
        ParamType::Uint(256),
        ParamType::Bytes,
        ParamType::Bool,
    ];

    let tokens: Vec<Token> = ethabi::decode(&abi, &input).unwrap();

    // let recipient = tokens[0].clone().into_address().unwrap();
    let amount_out = tokens[1].clone().into_uint().unwrap();
    // let amount_in_max = tokens[2].clone().into_uint().unwrap();
    let full_path = Bytes::from_token(tokens[3].clone()).unwrap().to_vec();
    let path: Vec<H160> = extract_path_from_v3(full_path, true);
    // let payer_is_user = tokens[4].clone().into_bool().unwrap();

    SwapUniversalRouterInfo {
        amount_in: U256::zero(),
        amount_out,
        path,
    }
}

pub fn extract_path_from_v3(full_path: Vec<u8>, is_reverse: bool) -> Vec<H160> {
    let mut path = Vec::new();
    let mut current_address = Vec::new();
    let mut index = 0;
    while index < full_path.len() {
        current_address.push(full_path[index]);
        if current_address.len() == 20 {
            path.push(Address::from_slice(&current_address));
            current_address = Vec::new();
            index += 4;
        } else {
            index += 1;
        }
    }

    // is_reverse = true for case V3_SWAP_EXACT_OUT
    if is_reverse {
        path.reverse()
    }
    path
}
