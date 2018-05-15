use std::env;
use std::path::PathBuf;
use std::error::Error;
use std::fs::{canonicalize, read_link};
use std::ffi::OsString;
use std;
use std::io::Read;
use log::LevelFilter;
use structopt::StructOpt;
use action::{Action, SubGitEnv};
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

        println!("Current Path: {:?}, Current Git DIR: {:?}", env::current_exe().unwrap(), git_os_dir);

        match git_os_dir {
            Some(git_os_path) => {
                let git_path = canonicalize(git_os_path).expect("Cannot open the GIT_DIR");
                if git_path.join("data").join("settings.toml").is_file() {
                    println!("In subgit repo");
                    ExecEnv::Subgit(SubGitEnv {
                        git_dir: git_path.clone(),
                        hook_path: git_path.join("data").join("hook"),
                    })
                } else {
                    println!("In upstream repo");
                    let hook_path = find_subgit_from_hook().expect("Cannot follow symlink");
                    let repo_path = hook_path.parent().unwrap().parent().unwrap();
                    if !repo_path
                        .join("data")
                        .join("settings.toml")
                        .is_file()
                    {
                        panic!("Cannot find subgit path!");
                    };
                    ExecEnv::Upstream(SubGitEnv {
                        git_dir: repo_path.to_owned(),
                        hook_path: hook_path.to_owned(),
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
            ExecEnv::Upstream(env) => Ok(Action::RequestSync(action::RequestSync {
                env,
                stdin: std::io::stdin().bytes().collect::<Result<Vec<u8>,_>>()?,
            })),
            ExecEnv::Subgit(env) => {
                let args: Vec<_> = iterable.into_iter().collect();
                let string_args: Vec<String> = args.iter()
                    .map(|v| v.clone().into().to_string_lossy().into_owned())
                    .collect();
                string_args.iter().for_each(|v| println!("Arg: {}", v));
                match args.len() {
                    2 => match string_args[1].as_str() {
                        "sync-all" => Ok(Action::SyncAll(action::SyncAll { env })),
                        "sync-refs" => {

                            let stdin_bytes = std::io::stdin().bytes().collect::<Result<Vec<u8>,_>>()?;

                            let s = match std::str::from_utf8(&stdin_bytes) {
                                Ok(v) => v,
                                Err(e) => panic!("Invalid UTF-8 sequence: {}", e),
                            };

                            let reqs = s.lines().map(|v| v.trim()).map(|line| -> Result<action::RefSyncRequest, Box<Error>> {
                                let entries = line.split(" ").collect::<Vec<&str>>();
                                entries.iter().for_each(|v| println!("Value: {}", v));
                                println!("OLD: {}", Oid::from_str(&entries[0])?);
                                println!("OLD: {}", Oid::from_str(&entries[1])?);
                                match entries[..] {
                                    [old_sha, new_sha, ref_name] => {
                                        println!("{} {} {}", old_sha, new_sha, ref_name);
                                        Ok(action::RefSyncRequest {
                                            ref_name: ref_name.to_string(),
                                            old_upstream_sha: Oid::from_str(old_sha)?,
                                            new_upstream_sha: Oid::from_str(new_sha)?,
                                        })
                                    },
                                    _ => Err(Box::new(StringError {
                                        message: "Bad args".to_owned()
                                    })),
                                }
                            }).collect::<Result<Vec<action::RefSyncRequest>, Box<Error>>>()?;

                            Ok(Action::SyncRefs(action::SyncRefs {
                                env,
                                requests: reqs,
                            }))
                        },
                        bad_arg => Err(Box::new(StringError {
                            message: format!("Invalid argument: '{}'", bad_arg).to_owned(),
                        })),
                    },
                    4 => Ok(Action::UpdateHook(action::UpdateHook {
                        env,
                        ref_name: string_args.get(1).unwrap().clone(),
                        old_sha: Oid::from_str(string_args.get(2).unwrap())?,
                        new_sha: Oid::from_str(string_args.get(3).unwrap())?,
                    })),
                    _ => Err(Box::new(StringError {
                        message: format!("Unknown argument structure: '{}'", string_args.join(" ")),
                    })),
                }
            }
            ExecEnv::Setup(path) => SetupRequest::from_iter(iterable).convert(path),
        }
    }
}
