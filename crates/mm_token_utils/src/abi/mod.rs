use ethers::contract::abigen;

abigen!(AvabotRouterAbigen, "src/abi/AvabotRouter.json");
abigen!(UniswapV2Router02Abigen, "src/abi/UniswapV2Router02.json");
abigen!(IUniswapV2PairAbigen, "src/abi/IUniswapV2Pair.json");
abigen!(UniswapV2FactoryAbigen, "src/abi/UniswapV2Factory.json");
abigen!(UniswapV3Router02Abigen, "src/abi/UniswapV3Router02.json");
abigen!(UniswapV3PoolAbigen, "src/abi/UniswapV3Pool.json");
abigen!(UniswapV3FactoryAbigen, "src/abi/UniswapV3Factory.json");
abigen!(QuoterV2Abigen, "src/abi/QuoterV2.json");
abigen!(MemeTokenAbigen, "src/abi/MemeToken.json");
abigen!(DisperseAbigen, "src/abi/Disperse.json");
abigen!(
    MemeTokenControllerAbigen,
    "src/abi/MemeTokenController.json"
);
