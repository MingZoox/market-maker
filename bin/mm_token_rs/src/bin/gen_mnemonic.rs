use mm_token_utils::{log::setup_logger, utils::random_mnemonic_phrase};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    setup_logger(None)?;

    let phrase = random_mnemonic_phrase();
    log::info!("Your mnemonic: \"{}\"", phrase);
    Ok(())
}
