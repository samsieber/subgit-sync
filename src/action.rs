use std::path::PathBuf;
use log::LevelFilter;
use std::error::Error;
use std::fs::File;
use fs2::FileExt;
use git2::Oid;

pub type RunResult = Result<(), Box<Error>>;

#[derive(Debug)]
pub struct SubGitEnv {
    pub git_dir: PathBuf,
    pub hook_path: PathBuf,
}

#[derive(Debug)]
pub struct SyncRef {
    pub ref_name: String,
    pub env: SubGitEnv,
}

#[derive(Debug)]
pub struct SyncAll {
    pub env: SubGitEnv,
}

#[derive(Debug)]
pub struct Setup {
    // Where we copy the binary from when we're done
    pub copy_from: PathBuf,

    // The git location
    pub upstream_git_location: PathBuf,
    pub subgit_git_location: PathBuf,

    // The path mapping
    pub upstream_map_path: PathBuf,
    pub subgit_map_path: Option<PathBuf>,

    // The log level to use
    pub log_level: LevelFilter,

    // The hook paths
    pub upstream_hook_path: Option<PathBuf>,
    pub subgit_hook_path: Option<PathBuf>,
}

#[derive(Debug)]
pub struct UpdateHook {
    pub env: SubGitEnv,
    pub ref_name: String,
    pub old_sha: Oid,
    pub new_sha: Oid,
}

#[derive(Debug)]
pub struct PassToSubgit {
    pub env: SubGitEnv,
    pub args: Vec<String>,
    pub stdin: Vec<u8>,
}

#[derive(Debug)]
pub enum Action {
    SyncRef(SyncRef),
    SyncAll(SyncAll),
    Setup(Setup),
    UpdateHook(UpdateHook),
    PassToSubgit(PassToSubgit),
}

impl PassToSubgit {
    fn run(self) -> RunResult {
        panic!("Not implemented");
    }
}

impl Setup {
    fn run(self) -> RunResult {
        let subgit_map_path = self.subgit_map_path
            .map(|v| v.to_string_lossy().to_string());
        let wrapped = ::model::WrappedSubGit::run_creation(
            self.subgit_git_location,
            self.upstream_git_location,
            self.upstream_map_path.to_str().unwrap(),
            subgit_map_path.as_ref().map(String::as_str),
            self.log_level,
            ::model::BinSource {
                location: self.copy_from,
                symlink: false,
            },
            self.subgit_hook_path,
            self.upstream_hook_path,
        )?;
        wrapped.update_all_from_upstream()?;

        panic!("Implementation not finished yet");
    }
}

impl UpdateHook {
    pub fn run(self) -> RunResult {
        println!("Running update");
        println!("Trying to open wrapped git: {:?}", ::fs::make_absolute(&self.env.git_dir));
        let wrapped = ::model::WrappedSubGit::open(self.env.git_dir)?;
        println!("Opened wrapped");
        println!("Trying to lock on {:?}", ::fs::make_absolute(wrapped.location.join("hook")));
        let file = File::open(wrapped.location.join("hook"))?;
        file.lock_exclusive()?;
        println!("Locked!");
        wrapped.push_ref_change_upstream(self.ref_name, self.old_sha, self.new_sha)?;

        Ok(())
    }
}


impl SyncAll {
    pub fn run(self) -> RunResult {
        let wrapped = ::model::WrappedSubGit::open(self.env.git_dir)?;
        let file = File::open(wrapped.location.join("hook"))?;
        file.lock_exclusive()?;

        wrapped.update_all_from_upstream();

        Ok(())
    }
}

impl Action {
    pub fn run(self) -> RunResult {
        println!("Running action: {:?}", &self);
        match self {
            Action::Setup(setup) => setup.run(),
            Action::UpdateHook(update) => update.run(),
            Action::SyncAll(sync_all) => sync_all.run(),
            _ => panic!("Implementation not finished"),
        }
    }
}
