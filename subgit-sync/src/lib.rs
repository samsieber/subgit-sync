#[macro_use]
extern crate log;
#[macro_use]
extern crate serde_derive;
extern crate structopt;

extern crate serde;
extern crate serde_json;

extern crate fs2;
extern crate git2;
extern crate hex;
extern crate libc;
extern crate log_panics;
extern crate nix;
extern crate simplelog;

use std::error::Error;

mod action;
mod cli;
mod fs;
mod git;
mod logging;
mod model;
mod util;

pub use crate::fs::make_absolute;
pub use crate::model::BinSource;
pub use crate::model::WrappedSubGit;
pub use crate::util::fork_into_child;
pub use crate::util::StringError;
pub use crate::cli::SetupRequest;

pub fn run() -> Result<(), failure::Error> {
    let exec_env = cli::ExecEnv::detect();
    let action = exec_env.parse_command(std::env::args())?;
    action.run()
}
