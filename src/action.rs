use std::path::PathBuf;
use log::LevelFilter;
use std::error::Error;
use std::fs::File;
use fs2::FileExt;
use git2::Oid;

pub type RunResult = Result<(), Box<Error>>;

pub struct SubGitEnv {
    pub git_dir: PathBuf,
    pub hook_path: PathBuf,
}

pub struct SyncRef {
    pub ref_name: String,
    pub env: SubGitEnv,
}

pub struct SyncAll {
    pub env: SubGitEnv,
}

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

pub struct UpdateHook {
    pub env: SubGitEnv,
    pub ref_name: String,
    pub old_sha: Oid,
    pub new_sha: Oid,
}

pub struct PassToSubgit {
    pub env: SubGitEnv,
    pub args: Vec<String>,
}

pub enum Action {
    SyncRef(SyncRef),
    SyncAll(SyncAll),
    Setup(Setup),
    UpdateHook(UpdateHook),
    PassToSubgit(PassToSubgit),
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
        )?;
        wrapped.update_all_from_upstream()?;

        panic!("Implementation not finished yet");
    }
}

impl UpdateHook {
    pub fn run(self) -> RunResult {
        let wrapped = ::model::WrappedSubGit::open(self.env.git_dir)?;
        let file = File::open("file.lock")?;
        file.lock_exclusive()?;

        wrapped.push_ref_change_upstream(self.ref_name, self.old_sha, self.new_sha)?;

        Ok(())
    }
}

impl Action {
    pub fn run(self) -> RunResult {
        match self {
            Action::Setup(setup) => setup.run(),
            _ => panic!("Implementation not finished"),
        }
    }
}
