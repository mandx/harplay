use chrono::Utc;
use fern::{
    colors::{Color, ColoredLevelConfig},
    Dispatch,
};
use log::Level;

pub fn setup_logging(level: Option<Level>) -> Result<(), fern::InitError> {
    let colors = ColoredLevelConfig::new()
        .info(Color::Cyan)
        .debug(Color::BrightWhite);

    Dispatch::new()
        .level(Level::Warn.to_level_filter())
        .level_for("harplay", level.unwrap_or(Level::Info).to_level_filter())
        .format(move |out, message, record| {
            out.finish(format_args!(
                "{} | {} | {} | {}",
                Utc::now().format("[%+]"),
                colors.color(record.level()),
                record.module_path().unwrap_or("::"),
                message
            ))
        })
        .chain(std::io::stderr())
        .apply()?;

    Ok(())
}
