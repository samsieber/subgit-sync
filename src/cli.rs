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
    if std::fs::symlink_metadata(&path)?.file_type().is_symlink() {
        Ok(read_link(&path)?)
    } else {
        Ok(path)
    }
}

#[derive(StructOpt)]
#[structopt()]
struct SetupRequest {
    // The git location
    upstream_git_location: String,
    subgit_git_location: String,

    // The path mapping
    upstream_map_path: String,

    #[structopt(short = "s", long = "subgit_map_path")]
    subgit_map_path: Option<String>,

    // The log level to use
    #[structopt(short = "l", long = "log_level")]
    log_level: Option<LevelFilter>,
    #[structopt(short = "f", long = "log_file", parse(from_os_str))]
    log_file: Option<PathBuf>,

    // The hook paths
    #[structopt(short = "U", long = "upstream_hook_path")]
    upstream_hook_path: Option<String>,
    #[structopt(short = "S", long = "subgit_hook_path")]
    subgit_hook_path: Option<String>,
    #[structopt(short = "r", long = "upstream_clone_url")]
    upstream_clone_url: Option<String>,
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
            log_file: self.log_file.unwrap_or(PathBuf::from("git_subgit_setup.log")),

            subgit_hook_path: self.subgit_hook_path.map(|v| PathBuf::from(v)),
            upstream_hook_path: self.upstream_hook_path.map(|v| PathBuf::from(v)),
            upstream_working_clone_url: self.upstream_clone_url,
        }))
    }
}

fn read_to_string<R : Read>(readable: &mut R) -> String {
    let mut s = String::new();
    readable.read_to_string(&mut s).unwrap();
    s
}

impl ExecEnv {
    pub fn detect() -> ExecEnv {
        let git_os_dir = env::var_os("GIT_DIR");

//        println!("Current Path: {:?}, Current Git DIR: {:?}", env::current_exe().unwrap(), git_os_dir);

        match git_os_dir {
            Some(git_os_path) => {
                let git_path = canonicalize(git_os_path).expect("Cannot open the GIT_DIR");
                if git_path.join("data").join("settings.toml").is_file() {
//                    eprintln!("In subgit repo");
                    ExecEnv::Subgit(SubGitEnv {
                        git_dir: git_path.clone(),
                        hook_path: git_path.join("data").join("hook"),
                    })
                } else {
//                    eprintln!("In upstream repo");
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
                stdin: read_to_string(&mut std::io::stdin()).into_bytes(),
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

                            let mut s = String::new();
                            std::io::stdin().read_to_string(&mut s).unwrap();

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
