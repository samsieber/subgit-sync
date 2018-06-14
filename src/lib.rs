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

pub fn run_import_test(id: &str, remote: &str, subdir: &str) -> Result<(), Box<Error>> {
    let top = Path::new("data").join(id);
    let subgit_path = top.join("subgit");
    let upstream_path = top.join("upstream");

    // Setup test data location, cleaning out old subgit
    fs::create_dir_all(&top)?;
    fs::remove_if_exists(&subgit_path)?;
    fs::remove_if_exists(&upstream_path)?;
    git::open_or_clone_bare(&upstream_path, remote);
    fs::remove_if_exists(upstream_path.join("hooks").join("post-receive"))?;

    let wrapped = model::WrappedSubGit::create_or_fail(subgit_path, upstream_path, subdir)?;

    wrapped.update_all_from_upstream()?;

    Ok(())
}
