use std::env;
use std::path::PathBuf;
use std::error::Error;
use std::fs::{canonicalize, read_link, symlink_metadata};
use std::ffi::OsString;
use log::LevelFilter;
use structopt::StructOpt;
use action::{Action, SubGitEnv};
use git2;
use git2::Oid;
use action;
use util::StringError;

pub enum ExecEnv {
    Subgit(SubGitEnv),
    Upstream(SubGitEnv),
    Setup(PathBuf),
}

fn find_subgit_from_hook() -> Result<PathBuf, Box<Error>> {
    let path = env::current_exe()?;
    Ok(read_link(path)?)
}

#[derive(StructOpt)]
#[structopt()]
struct SetupRequest {
    // The git location
    upstream_git_location: String,
    subgit_git_location: String,

    // The path mapping
    upstream_map_path: String,
    subgit_map_path: Option<String>,

    // The log level to use
    log_level: Option<LevelFilter>,

    // The hook paths
    upstream_hook_path: Option<String>,
    subgit_hook_path: Option<String>,
}

impl SetupRequest {
    fn convert(self, copy_from: PathBuf) -> Result<Action, Box<Error>> {
        Ok(Action::Setup(action::Setup {
            copy_from: copy_from,
            upstream_git_location: PathBuf::from(self.upstream_git_location),
            subgit_git_location: PathBuf::from(self.subgit_git_location),

            upstream_map_path: PathBuf::from(self.upstream_map_path),
            subgit_map_path: self.subgit_map_path.map(|v| PathBuf::from(v)),

            log_level: self.log_level.unwrap_or(LevelFilter::Debug),

            subgit_hook_path: self.subgit_hook_path.map(|v| PathBuf::from(v)),
            upstream_hook_path: self.upstream_hook_path.map(|v| PathBuf::from(v)),
        }))
    }
}

impl ExecEnv {
    pub fn detect() -> ExecEnv {
        let args: Vec<_> = env::args().collect();
        let git_os_dir = env::var_os("GIT_DIR");

        match git_os_dir {
            Some(git_os_path) => {
                let git_path = canonicalize(git_os_path).expect("Cannot open the GIT_DIR");
                if git_path.join("data").join("settings.toml").is_file() {
                    ExecEnv::Subgit(SubGitEnv {
                        git_dir: git_path,
                        hook_path: env::current_exe().expect("This is essential!"),
                    })
                } else {
                    let hook_path = find_subgit_from_hook().expect("Cannot follow symlink");
                    let subgit_repo =
                        git2::Repository::discover(&hook_path).expect("Cannot find subgit path");
                    if !subgit_repo
                        .path()
                        .join("data")
                        .join("settings.toml")
                        .is_file()
                    {
                        panic!("Cannot find subgit path!");
                    };
                    ExecEnv::Upstream(SubGitEnv {
                        git_dir: subgit_repo.path().to_owned(),
                        hook_path,
                    })
                }
            }
            None => {
                ExecEnv::Setup(env::current_exe().expect("Cannot read current executable path"))
            }
        }
    }

    pub fn parse_command<I>(self, iterable: I) -> Result<Action, Box<Error>>
    where
        I: IntoIterator,
        I::Item: Into<OsString> + Clone,
    {
        match self {
            ExecEnv::Upstream(env) => Ok(Action::PassToSubgit(action::PassToSubgit {
                env,
                args: iterable
                    .into_iter()
                    .map(|v| v.into().to_string_lossy().into_owned())
                    .collect(),
            })),
            ExecEnv::Subgit(env) => {
                let args: Vec<_> = iterable.into_iter().collect();
                let string_args: Vec<String> = args.iter()
                    .map(|v| v.clone().into().to_string_lossy().into_owned())
                    .collect();
                match args.len() {
                    1 => match string_args.first().unwrap().as_str() {
                        "sync-all" => Ok(Action::SyncAll(action::SyncAll { env })),
                        bad_arg => Err(Box::new(StringError {
                            message: format!("Invalid argument: '{}'", bad_arg).to_owned(),
                        })),
                    },
                    2 => match string_args.first().unwrap().as_str() {
                        "sync-branch" => Ok(Action::SyncRef(action::SyncRef {
                            env,
                            ref_name: string_args.get(1).unwrap().clone(),
                        })),
                        bad_arg => Err(Box::new(StringError {
                            message: format!("Invalid argument: '{}'", bad_arg).to_owned(),
                        })),
                    },
                    3 => Ok(Action::UpdateHook(action::UpdateHook {
                        env,
                        ref_name: string_args.get(0).unwrap().clone(),
                        old_sha: Oid::from_str(string_args.get(1).unwrap())?,
                        new_sha: Oid::from_str(string_args.get(1).unwrap())?,
                    })),
                    _ => Err(Box::new(StringError {
                        message: format!("Invalid argument: '{}'", string_args.join(" ")),
                    })),
                }
            }
            ExecEnv::Setup(path) => SetupRequest::from_iter(iterable).convert(path),
        }
    }
}
