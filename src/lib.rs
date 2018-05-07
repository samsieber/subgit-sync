#[macro_use]
extern crate log;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate structopt;

extern crate serde;
extern crate toml;

extern crate git2;
extern crate hex;
extern crate simplelog;

use std::error::Error;
use std::path::{Path};

mod model;
mod fs;
mod logging;
mod control;

pub use logging::setup_logging;
pub use model::WrappedSubGit;

pub fn run_import_test(id: &str, remote: &str, subdir: &str) -> Result<(), Box<Error>> {
    let top = Path::new("data").join(id);
    let subgit_path = top.join("subgit");
    let upstream_path = top.join("upstream");

    // Setup test data location, cleaning out old subgit
    fs::create_dir_all(&top)?;
    fs::remove_if_exists(&subgit_path)?;
    model::open_or_clone_bare(&upstream_path, remote);

    let wrapped = model::WrappedSubGit::create_or_fail(subgit_path, upstream_path, subdir)?;

    wrapped.update_all_from_upstream();

    Ok(())
}
