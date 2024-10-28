# avabot-meme-mm

## Notice

- MAKE SURE APPROVE TOKEN BEFORE USE THIS
  - uniswapv2
  - avabot_router (optional)
- Double check some important constants at `crates/mm_token_utils/src/constants`
  - NETWORKS
  - UNISWAP2_ROUTERS
  - UNISWAP3_ROUTERS
  - WRAPPED_NATIVE_TOKENS

- Config in `.env` file (check possible variables in `.env.example`)
```sh
# common fields
LISTEN_NETWORK=BLAST_SEPOLIA
TOKEN_ADDRESS=
ACTIVE_ROUTER=UNISWAP2_ROUTERS
TRADING_SLIPPAGE=1
TOKEN_BUY_TAX=0
TOKEN_SELL_TAX=0
```

## Requirements

### Early buy bot - normal mode
The program will load all wallets & balances into process, from `BUYER_MNEMONIC`. Then spam transactions to buy tokens `TOKEN_ADDRESS` from all wallets

```sh
# BUYER_MNEMONIC                   : mnemonic for buyer
# BUYER_WALLETS_COUNT              : number of wallets to use
# BUYER_SURPLUS_BALANCE            : eth amount keep in wallet after buying
cargo run -r -p mm_token_rs --bin buy_bot
```

### Auto sell bot
Trigger ASK whenever there is a BID on the market

```sh
# SELLER_MNEMONIC                  : mnemonic for seller
# SELLER_WALLETS_COUNT             : number of wallets to use
# AUTO_SELL_VOLUME_THRESHOLD       : minimum volume to trigger sell
# AUTO_SELL_MIN_PERCENT            : minimum percent of volume to trigger sell
# AUTO_SELL_MAX_PERCENT            : maximum percent of volume to trigger sell
# AUTO_SELL_MEMPOOL_LISTEN_ENABLED : enable mempool listen
# AUTO_SELL_EVENT_LISTEN_ENABLED   : enable event listen
cargo run -r -p mm_token_rs --bin sell_bot
```

### Auto buy bot
Trigger buy when catch sell event

```sh
# AUTO_BUYER_MNEMONIC                   : mnemonic for buyer
# AUTO_BUYER_WALLETS_COUNT              : number of wallets to use
# AUTO_BUYER_SURPLUS_BALANCE            : eth amount keep in wallet after buying
# FLOOR_PRICE                           : trigger buy if token price below this
# AUTO_BUY_MIN_PERCENT                  : auto buy min percent
# AUTO_BUY_MAX_PERCENT                  : auto buy max percent
cargo run -r -p mm_token_rs --bin auto_buy_bot
```

### Volume maker bot
Allocate significant ETH to the first address for market making, use a portion to buy, sell tokens and then transfer ETH to the next address, repeating the process to reach `maxWalletsCount` or until ETH is depleted.

Note: Config in `mm_config.json` file

```sh
cargo run -r -p mm_token_rs --bin market_make
```

### Launching new token
First Deployer initiating active trading: Buyer acquires tokens at block 0, followed by transferring all tokens and ETH to Seller's wallet.

```sh
# DEPLOYER_PRIVATE_KEY             : active trading wallet
# BUYER_MNEMONIC                   : mnemonic for buyer
# BUYER_WALLETS_COUNT              : number of wallets to use
# BUYER_SURPLUS_BALANCE            : eth amount keep in wallet after buying
# SELLER_MNEMONIC                  : mnemonic for seller
# SELLER_WALLETS_COUNT             : number of wallets to use
cargo run -r -p mm_token_rs --bin launching_token
```

### Api web server

Rocket requires to use `Rust nightly build`, so easy way to switch to `nightly build`:
```sh
rustup default nightly
rustup update && cargo update
```
Then: 

```sh
cargo run -r -p mm_token_rs --bin api_web_server
```

### Telegram Monitor bot

Telegram Monitor bot.

```sh
# TELEGRAM_BOT_TOKEN        : telegram bot token
cargo run -r -p telegram_bot_rs --bin telegram_monitor_bot
```

## More Utility Commands

#### Generate new mnemonic

```sh
cargo run -r -p mm_token_rs --bin gen_mnemonic
```

#### Check mnemonic

```sh
# CHECKED_MNEMONIC=                : check mnemonic wallet
# CHECKED_MNEMONIC_WALLET_COUNT=   : wallet count
cargo run -r -p mm_token_rs --bin check_mnemonic
```

#### Check buyer wallets

Check token balance & approval of all buyer wallets. Print the warning if a wallet has insufficient fund (to pay for gas fee) or invalid approval (allowance is less than token balance).

```sh
cargo run -r -p mm_token_rs --bin check_buyer_wallets_balances
```

#### Disperse ETH

```sh
# DISPERSE_ETH_PRIVATE_KEY=           : private key disperse wallet
# DISPERSE_ETH_MNEMONIC=              : target wallet mnemonic

# param1: DISPERSE_ETH_AMOUNT             -> eth amount disperse for each wallet
# param2: DISPERSE_ETH_WALLET_INDEX_FROM  -> start index wallet
# param3: DISPERSE_ETH_WALLET_INDEX_TO    -> end index wallet
cargo run -r -p mm_token_rs --bin disperse_eth 0.003 0 2
```

#### Disperse tokens

```sh
# DISPERSE_TOKEN_PRIVATE_KEY=         : private key disperse wallet
# DISPERSE_TOKEN_MNEMONIC=            : target wallet mnemonic

# param1: DISPERSE_TOKEN_WALLET_INDEX_FROM=  -> start index wallet
# param2: DISPERSE_TOKEN_WALLET_INDEX_TO=    -> end index wallet
# param3: DISPERSE_TOKEN_AMOUNT_MIN=         -> token amount min
# param4: DISPERSE_TOKEN_AMOUNT_MAX=         -> token amount max
cargo run -r -p mm_token_rs --bin disperse_tokens 2 4 1000 2000
```

#### Set whitelist buyer wallets

```sh
cargo run -r -p mm_token_rs --bin set_whitelist_buyer
```

#### Migrate token

Migrate `TOKEN_ADDRESS` from `BUYER_MNEMONIC` to `SELLER_MNEMONIC`.

```sh
cargo run -r -p mm_token_rs --bin migrate_token_buyer_to_seller
```

#### Migrate eth

Migrate `ETH` from `BUYER_MNEMONIC` to `SELLER_MNEMONIC`.

```sh
cargo run -r -p mm_token_rs --bin migrate_eth_buyer_to_seller
```

#### Dump all tokens

Dump all tokens of buyer wallets.

```sh
# param1: dump-interval-min -> the min rest time between two dumps (unit: seconds)
# param2: dump-interval-max -> the max time between two dumps (unit: seconds)
cargo run -r -p mm_token_rs --bin dump_all 100 200
```

#### Approve max to router

Approve for AMM router to spend token on `SELLER_MNEMONIC` wallets

```sh
# param1: APPROVE_SELLER_WALLET_INDEX_FROM= -> start index wallet
# param2: APPROVE_SELLER_WALLET_INDEX_TO=   -> end index wallet
cargo run -r -p mm_token_rs --bin approve_max_to_seller 0 1
```

#### Consolidate

- Consolidate `TOKEN_ADDRESS` from `MIGRATED_MNEMONIC` into 1 wallet `MIGRATION_WALLET`.

```sh
cargo run -r -p mm_token_rs --bin migrate_all_token
```

- Consolidate ETH from `MIGRATED_MNEMONIC` into 1 wallet `MIGRATION_WALLET`.

```sh
cargo run -r -p mm_token_rs --bin migrate_all_eth
```
