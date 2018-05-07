use std::env;
use std::path::PathBuf;
use std::error::Error;
use std::fs::{symlink_metadata, canonicalize, read_link};
use std::ffi::OsString;
use log::LevelFilter;
use structopt::StructOpt;

use std::error;
use std::fmt;

#[derive(Debug)]
pub struct StringError {
    message: String
}

impl fmt::Display for StringError {
    // This trait requires `fmt` with this exact signature.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // Write strictly the first element into the supplied output
        // stream: `f`. Returns `fmt::Result` which indicates whether the
        // operation succeeded or failed. Note that `write!` uses syntax which
        // is very similar to `println!`.
        write!(f, "{}", self.message)
    }
}

impl error::Error for StringError {
    fn description(&self) -> &str {
        self.message.as_ref()
    }

    fn cause(&self) -> Option<&error::Error> {
        None
    }
}

pub enum ExecEnv {
    Subgit(PathBuf),
    Upstream(PathBuf)
}

fn find_subgit_from_hook() -> Result<PathBuf, Box<Error>>{
    let path = env::current_exe()?;
    Ok(read_link(path)?)
}

#[derive(StructOpt)]
#[structopt()]
struct SetupRequest {
    upstream_path: String,
    local_path: String,
    log_level: LevelFilter,
    subgit_hook_path: Option<String>,
    upstream_hook_path: Option<String>,
}

impl SetupRequest {
    fn convert(self) -> Result<Action, Box<Error>> {
        Ok(Action::Setup {
            upstream_path: PathBuf::from(self.upstream_path),
            local_path: PathBuf::from(self.local_path),
            log_level: self.log_level,
            subgit_hook_path: self.subgit_hook_path.map(|v|  PathBuf::from(v)),
            upstream_hook_path: self.upstream_hook_path.map(|v| PathBuf::from(v)),
        })
    }
}

pub enum Action {
    SyncRef {
        ref_name: String,
        subgit: PathBuf,
    },
    SyncAll {
        subgit: PathBuf,
    },
    Setup{
        upstream_path: PathBuf,
        local_path: PathBuf,
        log_level: LevelFilter,
        subgit_hook_path: Option<PathBuf>,
        upstream_hook_path: Option<PathBuf>
    },
    Hook {
        subgit: PathBuf,
    },
    PassToSubgit {
        path: PathBuf,
        args: Vec<String>,
    }
}

impl ExecEnv {
    pub fn detect() -> Option<ExecEnv> {
        let args : Vec<_> = env::args().collect();
        let git_os_dir = env::var_os("GIT_DIR");

        match git_os_dir {
            Some(git_os_path) => {
                let git_path = canonicalize(git_os_path).expect("Cannot open the GIT_DIR");
                if git_path.join("data").join("settings.toml").is_file() {
                    Some(ExecEnv::Subgit(git_path))
                } else {
                    Some(ExecEnv::Upstream(find_subgit_from_hook().expect("Cannot follow symlink")))
                }
            },
            None => None
        }
    }

    pub fn parse_command<I>(exec_env: Option<ExecEnv>, iterable: I) -> Result<Action, Box<Error>>
    where
        I: IntoIterator,
        I::Item: Into<OsString> + Clone
    {
        match exec_env {
            Some(ExecEnv::Upstream(path)) => {
                Ok(Action::PassToSubgit {
                    path: path,
                    args: iterable.into_iter().map(|v| v.into().to_string_lossy().into_owned()).collect(),
                })
            }
            Some(ExecEnv::Subgit(path)) => {
                let args : Vec<_>= iterable.into_iter().collect();
                let string_args : Vec<String> = args.iter().map(|v| v.clone().into().to_string_lossy().into_owned()).collect();
                match args.len() {
                    0 => Ok(Action::Hook {
                        subgit: path,
                    }),
                    1 => match string_args.first().unwrap().as_str() {
                        "sync-all" => Ok(Action::SyncAll {
                            subgit: path,
                        }),
                        bad_arg => Err(Box::new(StringError { message: format!("Invalid argument: '{}'", bad_arg).to_owned() })),
                    },
                    2 => match string_args.first().unwrap().as_str() {
                        "sync-branch" => Ok(Action::SyncRef {
                            subgit: path,
                            ref_name: string_args.get(1).unwrap().clone()
                        }),
                        bad_arg => Err(Box::new(StringError { message: format!("Invalid argument: '{}'", bad_arg).to_owned() })),
                    },
                    _ =>  Err(Box::new(StringError { message: format!("Invalid argument: '{}'", string_args.join(" ")) }))
                }
            }
            None => SetupRequest::from_iter(iterable).convert()
        }
    }
}