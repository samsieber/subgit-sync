use std::path::Path;
use log::LevelFilter;
use simplelog::{CombinedLogger, TermLogger, WriteLogger, Config};
use std::fs::File;


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

pub fn configure_logging<P : AsRef<Path>>(stdout_level: LevelFilter, file_level: LevelFilter, file_path: &P){
    CombinedLogger::init(vec![
        TermLogger::new(stdout_level, Config::default()).unwrap(),
        WriteLogger::new(
            file_level,
            Config::default(),
            File::create(file_path.as_ref()).unwrap(),
        ),
    ]).unwrap();
    debug!("Logging started");
}