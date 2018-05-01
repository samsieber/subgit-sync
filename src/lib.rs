#[macro_use]
extern crate log;
extern crate simplelog;

use simplelog::*;

use std::fs::File;

pub fn setup_logging() {
  CombinedLogger::init(
        vec![
            TermLogger::new(LevelFilter::Info, Config::default()).unwrap(),
            WriteLogger::new(LevelFilter::Debug, Config::default(), File::create("my_rust_binary.log").unwrap()),
        ]
    ).unwrap();
    debug!("Logging started");
}