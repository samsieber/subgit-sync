extern crate git2;
extern crate hex;
#[macro_use]
extern crate log;
extern crate simplelog;

use git2::Repository;
use simplelog::*;
use std::fs::File;
use std::error::Error;
use std::path::{Path, PathBuf};

pub mod model;
mod fs;

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

pub fn open_or_clone_bare<P: AsRef<Path>>(path: P, url: &str) -> Repository {
    match Repository::open_bare(&path) {
        Ok(repo) => repo,
        Err(e) => {
            info!(
                "Couldn't open repo at {}, attempting clone from {}. Original error: {:?}",
                &path.as_ref().to_string_lossy(),
                &url,
                e
            );
            let mut builder = git2::build::RepoBuilder::new();
            match builder.bare(true).clone(url, path.as_ref()) {
                Ok(repo) => repo,
                Err(e) => panic!("failed to open or clone clone: {}", e),
            }
        }
    }
}

pub fn open_or_clone<P: AsRef<Path>>(path: P, url: &str) -> Repository {
    match Repository::open(&path) {
        Ok(repo) => repo,
        Err(e1) => match Repository::clone(url, &path) {
            Ok(repo) => repo,
            Err(e2) => panic!("failed to clone: {}, {}", e1, e2),
        },
    }
}

fn remove_if_exists(path: &Path) -> Result<(), Box<Error>> {
    if path.exists() {
        fs::remove_dir_all(&path)?;
    }
    Ok(())
}

pub fn run_import_test(id: &str, remote: &str, subdir: &str) -> Result<(), Box<Error>> {
    let top = Path::new("data").join(id);
    let subgit_path = top.join("subgit");
    let upstream_path = top.join("upstream");

    // Setup test data location, cleaning out old subgit
    fs::create_dir_all(&top)?;
    remove_if_exists(&subgit_path)?;
    open_or_clone_bare(&upstream_path, remote);

    let wrapped = model::WrappedSubGit::create_or_fail(subgit_path, upstream_path, subdir)?;

    wrapped.update_all_from_upstream();

    Ok(())
}
