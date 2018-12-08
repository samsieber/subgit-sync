use log::LevelFilter;
use simplelog::SimpleLogger;
use simplelog::{CombinedLogger, Config, WriteLogger};
use std::fs::OpenOptions;
use std::path::Path;

pub fn configure_logging<P: AsRef<Path>>(
    stdout_level: LevelFilter,
    file_level: LevelFilter,
    file_path: &P,
) {
    info!(
        "Logging file path: {:?}",
        &file_path.as_ref().to_string_lossy()
    );

    let f = OpenOptions::new()
        .create(true)
        .append(true)
        .open(file_path.as_ref())
        .unwrap();

    info!("File created");
    CombinedLogger::init(vec![
        SimpleLogger::new(stdout_level, Config::default()),
        WriteLogger::new(file_level, Config::default(), f),
    ])
    .unwrap();
    info!("Logging started");
}
