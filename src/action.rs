use std::path::PathBuf;
use log::LevelFilter;
use std::error::Error;
use std::fs::File;
use fs2::FileExt;
use git2::Oid;
use std::process::Command;
use std::env;
use git;
use std::process::Stdio;
use std::io::Write;
use std::path::Path;

pub type RunResult = Result<(), Box<Error>>;

#[derive(Debug)]
pub struct SubGitEnv {
    pub git_dir: PathBuf,
    pub hook_path: PathBuf,
}

#[derive(Debug)]
pub struct SyncRefs {
    pub requests: Vec<RefSyncRequest>,
    pub env: SubGitEnv,
}

#[derive(Debug)]
pub struct RefSyncRequest {
    pub ref_name: String,
    pub old_upstream_sha: Oid,
    pub new_upstream_sha: Oid,
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
    pub log_file: PathBuf,

    // The hook paths
    pub upstream_hook_path: Option<PathBuf>,
    pub subgit_hook_path: Option<PathBuf>,
    pub upstream_working_clone_url: Option<String>,
}

#[derive(Debug)]
pub struct UpdateHook {
    pub env: SubGitEnv,
    pub ref_name: String,
    pub old_sha: Oid,
    pub new_sha: Oid,
}

#[derive(Debug)]
pub struct RequestSync {
    pub env: SubGitEnv,
    pub stdin: Vec<u8>,
}

#[derive(Debug)]
pub enum Action {
    SyncRefs(SyncRefs),
    SyncAll(SyncAll),
    Setup(Setup),
    UpdateHook(UpdateHook),
    RequestSync(RequestSync),
}

impl RequestSync {
    fn run(self) -> RunResult {
        if git::get_git_options().unwrap_or(vec!()).iter().any(|x| x == "IGNORE_SUBGIT_UPDATE") {
            return Ok(());
        }
        let mut child = Command::new(&self.env.hook_path)
            .env_clear()
            .env("PATH", env::var("PATH").unwrap())
            .env("GIT_DIR",  self.env.hook_path.parent().unwrap().parent().unwrap().to_string_lossy().as_ref())
            .stdin(Stdio::piped())
            .arg("sync-refs")
            .spawn()
            .unwrap();
        child.stdin.as_mut().expect("Could not get stdin for child sync-refs child process").write_all(&self.stdin).unwrap();
        Ok(())
    }
}

pub fn lock<P: AsRef<Path>>(root: P) -> RunResult{
    info!("Trying to lock on {:?}", ::fs::make_absolute(&root.as_ref().join("data/lock")));
    let file = File::open(root.as_ref().join("data/lock"))?;
    file.lock_exclusive()?;
    info!("Locked!");
    Ok(())
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
            self.log_file,
            ::model::BinSource {
                location: self.copy_from,
                symlink: false,
            },
            self.subgit_hook_path,
            self.upstream_hook_path,
            self.upstream_working_clone_url,
        )?;
        wrapped.import_initial_empty_commits();
        wrapped.update_all_from_upstream()?;

        Ok(())
    }
}

impl UpdateHook {
    pub fn run(self) -> RunResult {
        let wrapped = ::model::WrappedSubGit::open(self.env.git_dir)?;

        info!("Opened Wrapped");
        lock(&wrapped.location)?;
        info!("Running update");
        wrapped.update_self();
        wrapped.push_ref_change_upstream(self.ref_name, self.old_sha, self.new_sha)?;

        Ok(())
    }
}


impl SyncAll {
    pub fn run(self) -> RunResult {
        let wrapped = ::model::WrappedSubGit::open(self.env.git_dir)?;

        info!("Opened Wrapped");
        lock(&wrapped.location)?;
        info!("Running Sync All");

        wrapped.update_self();
        wrapped.update_all_from_upstream()?;

        Ok(())
    }
}

impl SyncRefs {
    pub fn run(self) -> RunResult {
        super::util::fork_into_child();

        let wrapped = ::model::WrappedSubGit::open(self.env.git_dir)?;

        info!("Opened Wrapped");
        lock(&wrapped.location)?;
        info!("Running Sync Refs");

        wrapped.update_self();
        self.requests.into_iter()
            .filter(|req| git::is_applicable(&req.ref_name))
            .for_each(|request| {
                wrapped.import_upstream_commits(
                    &request.ref_name,
                    git::optionify_sha(request.old_upstream_sha),
                    git::optionify_sha(request.new_upstream_sha),
                );
            });

        Ok(())
    }
}

impl Action {
    pub fn run(self) -> RunResult {
//        println!("Running action: {:?}", &self);
        match self {
            Action::Setup(setup) => setup.run(),
            Action::UpdateHook(update) => update.run(),
            Action::SyncAll(sync_all) => sync_all.run(),
            Action::SyncRefs(sync_refs) => sync_refs.run(),
            Action::RequestSync(request_sync) => request_sync.run(),
        }
    }
}
