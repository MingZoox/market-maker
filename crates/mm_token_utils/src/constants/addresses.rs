use std::str::FromStr;

use ethers::prelude::Lazy;
use ethers::types::Address;

pub static ZERO_ADDRESS: Lazy<Address> =
    Lazy::new(|| Address::from_str("0x0000000000000000000000000000000000000000").unwrap());
