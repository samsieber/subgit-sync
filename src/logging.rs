use std::path::Path;
use log::LevelFilter;
use simplelog::{CombinedLogger, Config, TermLogger, WriteLogger};
use std::fs::File;
use simplelog::SimpleLogger;

pub fn setup_logging() {
    CombinedLogger::init(vec![
        TermLogger::new(LevelFilter::Info, Config::default()).unwrap(),
        WriteLogger::new(
            LevelFilter::Debug,
            Config::default(),
            File::create("my_rust_binary.log").unwrap(),
        ),
    ]).unwrap();
    debug!("Logging started");
}

pub fn configure_logging<P: AsRef<Path>>(
    stdout_level: LevelFilter,
    file_level: LevelFilter,
    file_path: &P,
) {
    println!("Logging file path: {:?}", &file_path.as_ref().to_string_lossy());

    let mut f = File::create(file_path.as_ref()).unwrap();

    println!("File created");
    CombinedLogger::init(vec![
        SimpleLogger::new(stdout_level, Config::default()),
        WriteLogger::new(
            file_level,
            Config::default(),
            f,
        ),
    ]).unwrap();
    debug!("Logging started");
}
