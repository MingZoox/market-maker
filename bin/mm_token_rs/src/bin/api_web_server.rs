use mm_token_rs::core::ApiService;
use mm_token_rs::types::{Buyers, Deployer, LaunchStatus, MarketMakers, NetworkStatus, Sellers};
use mm_token_utils::log::setup_logger;
use rocket::serde::json::Json;
use rocket::{get, launch, post, routes};

#[launch]
fn rocket() -> _ {
    dotenv::dotenv().ok();
    let _ = setup_logger(None);
    rocket::build()
        .configure(rocket::Config::figment().merge(("port", 8000)))
        .mount("/", routes![network_status])
        // .mount("/", routes![deployment_checklist])
        .mount("/", routes![deployer])
        .mount("/", routes![launch_process])
        .mount("/", routes![buyers])
        .mount("/", routes![auto_buyers])
        .mount("/", routes![sellers])
        .mount("/", routes![market_makers])
}

// APIs
#[get("/api/network_status")]
async fn network_status() -> Json<NetworkStatus> {
    let api_service = ApiService::new();
    let network_status = api_service.get_network_status().await;
    log::info!("[/api/network_status] Response: {:#?}", network_status);
    Json(network_status)
}

// #[get("/api/deployment_checklist")]
// async fn deployment_checklist() -> Json<DeploymentChecklist> {
//     let api_service = ApiService::new();
//     let deployment_checklist = api_service.get_deployment_checklist().await;
//     log::info!(
//         "[/api/deployment_checklist] Response: {:#?}",
//         deployment_checklist
//     );
//     Json(deployment_checklist)
// }

#[get("/api/deployer")]
async fn deployer() -> Json<Deployer> {
    let api_service = ApiService::new();
    let deployer = api_service.get_deployer().await;
    log::info!("[/api/deployer] Response: {:#?}", deployer);
    Json(deployer)
}

#[get("/api/buyers")]
async fn buyers() -> Json<Buyers> {
    let api_service = ApiService::new();
    let buyers = api_service.get_buyers().await;
    log::info!("[/api/buyers] Response: {:#?}", buyers);
    Json(buyers)
}

#[get("/api/auto_buyers")]
async fn auto_buyers() -> Json<Buyers> {
    let api_service = ApiService::new();
    let auto_buyers = api_service.get_auto_buyers().await;
    log::info!("[/api/auto_buyers] Response: {:#?}", auto_buyers);
    Json(auto_buyers)
}

#[get("/api/sellers")]
async fn sellers() -> Json<Sellers> {
    let api_service = ApiService::new();
    let sellers = api_service.get_sellers().await;
    log::info!("[/api/sellers] Response: {:#?}", sellers);
    Json(sellers)
}

#[get("/api/market_makers")]
async fn market_makers() -> Json<MarketMakers> {
    let api_service = ApiService::new();
    let market_makers = api_service.get_market_makers().await;
    log::info!("[/api/market_makers] Response: {:#?}", market_makers);
    Json(market_makers)
}

#[post("/api/launch")]
async fn launch_process() -> Json<LaunchStatus> {
    let api_service = ApiService::new();
    let launch_status = api_service.launch_process().await;
    log::info!("[/api/launch] Response: {:#?}", launch_status);
    Json(launch_status)
}
