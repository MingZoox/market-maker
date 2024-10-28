use mm_token_rs::core::MigrationService;
use mm_token_utils::log::setup_logger;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();
    setup_logger(None)?;

    let migration_service = MigrationService::new();
    migration_service.migrate_all_token().await?;
    Ok(())
}
