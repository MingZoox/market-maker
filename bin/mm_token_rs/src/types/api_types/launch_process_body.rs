use ethers::types::U256;
use serde::Deserialize;

#[derive(Debug, PartialEq, Eq, Deserialize)]
pub struct LaunchProcessBody {
    add_liquidity_token_balance: U256,
    add_liquidity_eth_balance: U256,
}

// #[derive(Debug)]
// enum Error {
//     Io(io::Error),
//     Parse,
// }

// impl<'a> FromData<'a> for LaunchProcessBody {
//     type Error = Error;

//     async fn from_data(_: &'a Request<'_>, data: Data<'a>) -> data::Outcome<'a, Self> {
//         use Error::*;

//         // Read the data into a string.
//         let string = match data.open().into_string().await {
//             Ok(string) => string,
//             Err(e) => return Failure((Status::InternalServerError, Io(e))),
//         };

//         // Split the string into two pieces at ':'.
//         let parts: Vec<&str> = string.split(',').collect();
//         if parts.len() != 2 {
//             return Failure((Status::UnprocessableEntity, Parse));
//         }

//         // Parse the values into U256.
//         let add_liquidity_token_balance: U256 = match parts[0].parse() {
//             Ok(balance) => balance,
//             Err(_) => return Failure((Status::UnprocessableEntity, Parse)),
//         };
//         let add_liquidity_eth_balance: U256 = match parts[1].parse() {
//             Ok(balance) => balance,
//             Err(_) => return Failure((Status::UnprocessableEntity, Parse)),
//         };

//         // Create the LaunchProcessBody instance.
//         let body = LaunchProcessBody {
//             add_liquidity_token_balance,
//             add_liquidity_eth_balance,
//         };

//         // Return the success outcome with the parsed body.
//         Success(body)
//     }
// }
