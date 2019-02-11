use crate::git;
use fs2::FileExt;
use git2::Oid;
use hex;
use log::LevelFilter;
use std::env;
use std::fs;
use std::fs::File;
use std::path::Path;
use std::path::PathBuf;

pub type RunResult = Result<(), failure::Error>;

pub trait RefFilter {
    fn matches<R: AsRef<str>>(&self, ref_name: R) -> bool;
}

impl RefFilter for Vec<String> {
    fn matches<R: AsRef<str>>(&self, ref_name: R) -> bool {
        self.iter().any(|v| ref_name.as_ref().starts_with(v))
    }
}

impl<'a> RefFilter for &'a Vec<String> {
    fn matches<R: AsRef<str>>(&self, ref_name: R) -> bool {
        self.iter().any(|v| ref_name.as_ref().starts_with(v))
    }
}

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

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct EnvDetect {
    pub name: String,
    pub value: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct UpdateWhitelist {
    pub path: PathBuf,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum RecursionDetection {
    Disabled,
    UsePushOptions,
    EnvBased(EnvDetect),
    UpdateWhitelist(UpdateWhitelist),
}

pub struct RecursionStatus {
    pub is_recursing: bool,
    pub reason: String,
}

pub trait PushListener {
    fn pre_push<S: AsRef<str>>(&self, ref_name: S, sha: Oid);
    fn post_push<S: AsRef<str>>(&self, ref_name: S, sha: Oid);
}

fn convert_ref_name<S: AsRef<str>>(name: S) -> String {
    name.as_ref().replace("/", ":")
}

fn get_file_name<N: AsRef<str>>(ref_name: N, sha: Oid) -> String {
    format!(
        "{}-{}",
        convert_ref_name(ref_name),
        hex::encode(sha.as_bytes())
    )
}

impl UpdateWhitelist {
    pub fn get_handle<N: AsRef<str>>(&self, ref_name: N, sha: Oid) -> PathBuf {
        return self.path.join(get_file_name(ref_name, sha));
    }
}

impl<'a> PushListener for &'a RecursionDetection {
    fn pre_push<S: AsRef<str>>(&self, ref_name: S, sha: Oid) {
        match self {
            RecursionDetection::UpdateWhitelist(update_whitelist) => {
                let handle = update_whitelist.get_handle(ref_name, sha);
                info!(
                    "Creating whitelist file for recursion detection: {}",
                    &handle.to_string_lossy()
                );
                File::create(handle).unwrap();
            }
            _ => {}
        }
    }

    fn post_push<S: AsRef<str>>(&self, ref_name: S, sha: Oid) {
        match self {
            RecursionDetection::UpdateWhitelist(update_whitelist) => {
                let handle = update_whitelist.get_handle(ref_name, sha);
                fs::remove_file(handle).unwrap();
            }
            _ => {}
        }
    }
}

impl RecursionDetection {
    pub fn get_push_opts(&self) -> Option<Vec<String>> {
        info!("Using push opts from {:?}", &self);
        match self {
            &RecursionDetection::UsePushOptions => Some(vec!["IGNORE_SUBGIT_UPDATE".to_string()]),
            &RecursionDetection::EnvBased(ref _env_detect) => None,
            &RecursionDetection::Disabled => None,
            &RecursionDetection::UpdateWhitelist(ref _update_whitelist) => None,
        }
    }

    pub fn detect_recursion(&self) -> RecursionStatus {
        match &self {
            &RecursionDetection::Disabled => RecursionStatus {
                is_recursing: false,
                reason: "Disabled".to_string(),
            },
            &RecursionDetection::UsePushOptions => {
                if git::get_git_options()
                    .unwrap_or(vec![])
                    .iter()
                    .any(|x| x == "IGNORE_SUBGIT_UPDATE")
                {
                    RecursionStatus {
                        is_recursing: true,
                        reason: format!(
                            "Found 'IGNORE_SUBGIT_UPDATE' as a git push option env variable value"
                        ),
                    }
                } else {
                    RecursionStatus {
                    is_recursing: false,
                    reason: format!("Didn't find 'IGNORE_SUBGIT_UPDATE' as a git push option env variable value")
                }
                }
            }
            &RecursionDetection::EnvBased(ref env_detect) => {
                if let Ok(value) = env::var(&env_detect.name) {
                    if value == env_detect.value {
                        RecursionStatus {
                            is_recursing: true,
                            reason: format!(
                                "Found variable {} with value {}",
                                env_detect.name, env_detect.value
                            ),
                        }
                    } else {
                        RecursionStatus {
                            is_recursing: false,
                            reason: format!(
                                "Found variable {} with value {} (needed {} as the value)",
                                env_detect.name, value, env_detect.value
                            ),
                        }
                    }
                } else {
                    RecursionStatus {
                        is_recursing: false,
                        reason: format!("Didn't find an env variable named {}", env_detect.name),
                    }
                }
            }
            &RecursionDetection::UpdateWhitelist(ref whitelist) => {
                if let Some(new_sha) = env::args().nth(3) {
                    let ref_name = env::args().nth(1).unwrap();
                    let path = whitelist
                        .get_handle(ref_name, Oid::from_str(&new_sha).unwrap_or(git::no_sha()));
                    if path.exists() {
                        RecursionStatus {
                            is_recursing: true,
                            reason: format!(
                                "Found file {} matching update request",
                                path.to_string_lossy()
                            ),
                        }
                    } else {
                        RecursionStatus {
                            is_recursing: false,
                            reason: format!(
                                "Could not find file {} matching update request",
                                path.to_string_lossy()
                            ),
                        }
                    }
                } else {
                    RecursionStatus {
                        is_recursing: false,
                        reason: format!("No update hook detected."),
                    }
                }
            }
        }
    }
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
    pub upstream_hook_path: PathBuf,
    pub subgit_hook_path: PathBuf,
    pub upstream_working_clone_url: Option<String>,
    pub subgit_working_clone_url: Option<String>,

    // recursion detection
    pub recursion_detection: RecursionDetection,

    // ref matching
    pub filters: Vec<String>,
}

#[derive(Debug)]
pub struct UpdateHook {
    pub env: SubGitEnv,
    pub ref_name: String,
    pub old_sha: Oid,
    pub new_sha: Oid,
}

#[derive(Debug)]
pub enum Action {
    SyncRefs(SyncRefs),
    SyncAll(SyncAll),
    Setup(Setup),
    UpdateHook(UpdateHook),
}

pub fn lock<P: AsRef<Path>>(root: P) -> Result<File, failure::Error> {
    info!(
        "Trying to lock on {:?}",
        crate::fs::make_absolute(&root.as_ref().join("data/lock"))
    );
    let file = fs::OpenOptions::new()
        .read(true)
        .open(root.as_ref().join("data/lock"))?;
    file.lock_exclusive()?;
    info!("Locked!");
    Ok(file)
}

impl Setup {
    fn run(self) -> RunResult {
        let subgit_map_path = self
            .subgit_map_path
            .map(|v| v.to_string_lossy().to_string());
        let mut wrapped = crate::model::WrappedSubGit::run_creation(
            self.subgit_git_location,
            self.upstream_git_location,
            self.upstream_map_path.to_str().unwrap(),
            subgit_map_path.as_ref().map(String::as_str),
            self.log_level,
            self.log_file,
            crate::model::BinSource {
                location: self.copy_from,
                symlink: false,
            },
            self.subgit_hook_path,
            self.subgit_working_clone_url,
            self.upstream_hook_path,
            self.upstream_working_clone_url,
            self.recursion_detection,
            self.filters,
        )?;
        wrapped.import_initial_empty_commits();
        wrapped.update_all_from_upstream()?;

        Ok(())
    }
}

fn empty(_filters: &Vec<String>) {}

impl UpdateHook {
    pub fn run(self) -> RunResult {
        let maybe_wrapped = crate::model::WrappedSubGit::open(self.env.git_dir, Some(empty))?;

        if let Some(mut wrapped) = maybe_wrapped {
            info!("Opened Wrapped");
            info!("Running update");
            wrapped.update_self();
            wrapped.push_ref_change_upstream(self.ref_name, self.old_sha, self.new_sha)?;

            Ok(())
        } else {
            Ok(())
        }
    }
}

impl SyncAll {
    pub fn run(self) -> RunResult {
        let maybe_wrapped = crate::model::WrappedSubGit::open(self.env.git_dir, Some(empty))?;

        if let Some(mut wrapped) = maybe_wrapped {
            info!("Running Sync All");

            wrapped.update_self();
            wrapped.update_all_from_upstream()?;
            Ok(())
        } else {
            Ok(())
        }
    }
}

impl SyncRefs {
    pub fn run(self) -> RunResult {
        let maybe_wrapped = crate::model::WrappedSubGit::open(
            &self.env.git_dir,
            Some(|filters: &Vec<String>| {
                let ref_names: Vec<_> = (&self.requests)
                    .iter()
                    .filter(|req| filters.matches(&req.ref_name))
                    .map(|req| &req.ref_name)
                    .collect();

                println!("Syncing refs: {:?}", ref_names);

                super::util::fork_into_child();
            }),
        )?;

        if let Some(mut wrapped) = maybe_wrapped {
            info!("Running Sync Refs");

            wrapped.update_self();
            self.requests
                .into_iter()
                .for_each(|request| {
                    if wrapped.filters.matches(&request.ref_name){
                        wrapped.import_upstream_commits(
                            &request.ref_name,
                            git::optionify_sha(request.old_upstream_sha),
                            git::optionify_sha(request.new_upstream_sha),
                        );
                    }
                });
            Ok(())
        } else {
            Ok(())
        }
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
        }
    }
}
