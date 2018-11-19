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
extern crate simplelog;
extern crate nix;
extern crate libc;
extern crate log_panics;

use std::error::Error;

mod model;
mod fs;
mod logging;
mod action;
mod cli;
mod util;
mod git;

pub use crate::model::WrappedSubGit;
pub use crate::util::fork_into_child;
pub use crate::model::BinSource;
pub use crate::util::StringError;
pub use crate::fs::make_absolute;

pub fn run() -> Result<(), Box<Error>> {
    let exec_env = cli::ExecEnv::detect();
    let action = exec_env.parse_command(std::env::args())?;
    action.run()
}