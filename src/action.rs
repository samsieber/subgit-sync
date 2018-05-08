use std::path::PathBuf;
use log::LevelFilter;
use std::error::Error;

pub type RunResult = Result<(), Box<Error>>;

pub struct SyncRef {
    pub ref_name: String,
    pub subgit: PathBuf,
}

pub struct SyncAll {
    pub subgit: PathBuf,
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

pub struct Hook {
    pub subgit: PathBuf,
}

pub struct PassToSubgit {
    pub path: PathBuf,
    pub args: Vec<String>,
}

pub enum Action {
    SyncRef(SyncRef),
    SyncAll(SyncAll),
    Setup(Setup),
    Hook(Hook),
    PassToSubgit(PassToSubgit),
}

impl Setup {
    fn run(self) -> RunResult {
        let subgit_map_path = self.subgit_map_path.map(|v| v.to_string_lossy().to_string());
        let wrapped = ::model::WrappedSubGit::run_creation(
            self.subgit_git_location,
            self.upstream_git_location,
            self.upstream_map_path.to_str().unwrap(),
            subgit_map_path.as_ref().map(String::as_str),
            self.log_level,
        );

        panic!("Implementation not finished yet");
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
