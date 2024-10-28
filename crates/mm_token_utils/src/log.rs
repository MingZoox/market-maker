use std::str::FromStr;

use fern::colors::{Color, ColoredLevelConfig};

use crate::env::get_env;

pub fn setup_logger(
    levels: Option<Vec<(String, log::LevelFilter)>>,
) -> Result<(), log::SetLoggerError> {
    let colors = ColoredLevelConfig {
        trace: Color::Cyan,
        debug: Color::Magenta,
        info: Color::Green,
        warn: Color::Red,
        error: Color::BrightRed,
    };

    let cargo_log_level_str = get_env("CARGO_LOG_LEVEL", Some("INFO".to_string()));
    let cargo_pkg_name = get_env("CARGO_PKG_NAME", None);
    let cargo_bin_name = current_bin_name().unwrap_or(cargo_pkg_name.clone());

    let mut dispatch = fern::Dispatch::new()
        .format(move |out, message, record| {
            out.finish(format_args!(
                "{}[{}] {}",
                chrono::Local::now().format("[%H:%M:%S.%f]"),
                colors.color(record.level()),
                message
            ))
        })
        .chain(std::io::stdout())
        .level(log::LevelFilter::Warn)
        .level_for(
            cargo_pkg_name,
            log::LevelFilter::from_str(&cargo_log_level_str).expect("CARGO_LOG_LEVEL invalid"),
        )
        .level_for(
            cargo_bin_name,
            log::LevelFilter::from_str(&cargo_log_level_str).expect("CARGO_LOG_LEVEL invalid"),
        );
    if let Some(levels) = levels {
        for (module, level) in levels {
            dispatch = dispatch.level_for(module, level);
        }
    }

    dispatch.apply()?;
    Ok(())
}

fn current_bin_name() -> Option<String> {
    std::env::current_exe()
        .ok()?
        .file_name()?
        .to_str()?
        .to_owned()
        .into()
}
