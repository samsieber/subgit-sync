#[macro_use]
extern crate log;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate structopt;

extern crate serde;
extern crate toml;

extern crate fs2;
extern crate git2;
extern crate hex;
extern crate simplelog;
extern crate nix;
extern crate libc;

use std::error::Error;
use std::path::Path;

mod model;
mod fs;
mod logging;
mod action;
mod cli;
mod util;
mod git;

pub use model::WrappedSubGit;
pub use util::fork_into_child;
pub use model::BinSource;
pub use util::StringError;
pub use fs::make_absolute;

pub fn run() -> Result<(), Box<Error>> {
    let exec_env = cli::ExecEnv::detect();
    let action = exec_env.parse_command(std::env::args())?;
    action.run()
}