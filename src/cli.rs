use crate::action;
use crate::action::EnvDetect;
use crate::action::{Action, SubGitEnv};
use crate::make_absolute;
use crate::model::settings::SETTINGS_FILE;
use crate::util::StringError;
use git2::Oid;
use log::LevelFilter;
use std;
use std::env;
use std::error::Error;
use std::ffi::OsString;
use std::fs::{canonicalize, read_link};
use std::io::Read;
use std::path::PathBuf;
use structopt::clap::AppSettings;
use structopt::StructOpt;

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

fn parse_env_base_recursion_detection(input: &&str) -> EnvDetect {
    let mut iter = input.splitn(2, ":");
    let env_variable_name = iter.next().unwrap();
    let env_variable_value = iter.next().unwrap();
    EnvDetect {
        name: env_variable_name.to_string(),
        value: env_variable_value.to_string(),
    }
}

fn str_to_vec(input: String) -> Vec<String> {
    let iter = input.split(",");
    iter.map(|v| v.to_owned()).collect()
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(
            super::parse_env_base_recursion_detection(&"GL_USERNAME:git"),
            super::EnvDetect {
                name: "GL_USERNAME".to_string(),
                value: "git".to_string(),
            }
        );
    }
}

/// Installs git hooks to republish a path of repository (henceforth: upstream)
/// as it's own top-level repository (henceforth: subgit) and synchronize commits between them,
/// using the upstream as the source of truth
///
/// It's designed to be used on repositories that reside on the same filesystem, for which you
/// have admin access.
///
/// It places an server-side update hook in the subgit repo, and a server-side post-receive hook
/// in the upstream repo. The update hooks synchronously exports commits from the subgit repo
/// to the upstream repo, refusing the push if the upstream cannot be updated. The upstream
/// hook asynchronously requests the subgit to import the newly pushed commits
#[derive(StructOpt)]
#[structopt(raw(global_settings = "&[AppSettings::DeriveDisplayOrder]"))]
struct SetupRequest {
    /// The location of the bare upstream repository on disk
    upstream_git_location: String,
    /// The location of the bare subgit repository on disk
    subgit_git_location: String,

    /// The path in the upstream repository to republish as the root in the subgit repository
    upstream_map_path: String,

    /// The path in the subgit repo to place the republished files from upstream
    /// Defaults to the root of the repository
    #[structopt(short = "p", long = "subgit_map_path")]
    subgit_map_path: Option<String>,

    /// The log level to use when logging to file from the hooks
    #[structopt(short = "l", long = "log_level")]
    log_level: Option<LevelFilter>,
    /// The path of the log file to write to during setup
    #[structopt(short = "f", long = "log_file", parse(from_os_str))]
    log_file: Option<PathBuf>,

    /// The hook path to use in the upstream repository
    #[structopt(
        short = "H",
        long = "upstream_hook_path",
        default_value = "hooks/post-receive",
        parse(from_os_str)
    )]
    upstream_hook_path: PathBuf,
    /// The hook path to use in the subgit repository
    #[structopt(
        short = "h",
        long = "subgit_hook_path",
        default_value = "hooks/update",
        parse(from_os_str)
    )]
    subgit_hook_path: PathBuf,

    /// Specify an external url to push changes to, when exporting commits to the upstream from the subgit
    /// If not set, uses the file path to the upstream repo
    #[structopt(short = "U", long = "upstream_working_clone_url")]
    upstream_working_clone_url: Option<String>,

    /// Specify an external url to push changes to, when import commits in the subgit from the upstream
    /// If not set, uses a modified subgit bare repo that bypasses the server hooks
    #[structopt(short = "u", long = "subgit_working_clone_url")]
    subgit_working_clone_url: Option<String>,

    /// Specifies an environment variable name and value to look for when trying to detect recursive hook calls
    /// Defaults to using the --push-option added in git 2.10
    /// The value must be in the form of ENV_NAME:ENV_VALUE
    /// For example, for gitlab servers, you'd most likely use 'GL_USERNAME:git' as the value
    #[structopt(
        short = "r",
        long = "env_based_recursion_detection",
        conflicts_with = "use_whitelist_recursion_detection",
        parse(from_str = "parse_env_base_recursion_detection")
    )]
    env_based_recursion_detection: Option<EnvDetect>,

    /// Disables recursive hook call checking
    /// This cannot be used with a custom subgit_working_clone_url due to the infinite recursion that occurs
    /// when both the upstream hook and subgit hook are triggered during synchronization
    #[structopt(
        short = "w",
        long = "use_whitelist_recursion_detection",
        conflicts_with = "env_based_recursion_detection"
    )]
    disable_recursion_detection: bool,

    /// Only operate on the refs that start with these values - pass in a comma separated list
    #[structopt(short = "m", long = "match_ref", default_value = "refs/heads/,HEAD")]
    match_ref: String,
}

impl SetupRequest {
    fn convert(self, copy_from: PathBuf) -> Result<Action, Box<Error>> {
        let recursion_detection = if self.disable_recursion_detection {
            action::RecursionDetection::UpdateWhitelist(action::UpdateWhitelist {
                path: make_absolute(
                    PathBuf::from(&self.subgit_git_location)
                        .join("data")
                        .join("whitelist"),
                )
                .unwrap(),
            })
        } else {
            match self.env_based_recursion_detection {
                Some(env_detect) => action::RecursionDetection::EnvBased(env_detect),
                None => action::RecursionDetection::UsePushOptions,
            }
        };

        Ok(Action::Setup(action::Setup {
            copy_from,
            upstream_git_location: PathBuf::from(self.upstream_git_location),
            subgit_git_location: PathBuf::from(self.subgit_git_location),

            upstream_map_path: PathBuf::from(self.upstream_map_path),
            subgit_map_path: self.subgit_map_path.map(|v| PathBuf::from(v)),

            log_level: self.log_level.unwrap_or(LevelFilter::Debug),
            log_file: self
                .log_file
                .unwrap_or(PathBuf::from("git_subgit_setup.log")),

            subgit_hook_path: self.subgit_hook_path,
            subgit_working_clone_url: self.subgit_working_clone_url,
            upstream_hook_path: self.upstream_hook_path,
            upstream_working_clone_url: self.upstream_working_clone_url,

            recursion_detection,

            filters: str_to_vec(self.match_ref),
        }))
    }
}

#[allow(unused)]
fn read_to_string<R: Read>(readable: &mut R) -> String {
    let mut s = String::new();
    readable.read_to_string(&mut s).unwrap();
    s
}

impl ExecEnv {
    pub fn detect() -> ExecEnv {
        let git_os_dir = env::var_os("GIT_DIR");
        let gl_username = env::var_os("GL_USERNAME");

        let in_hook = git_os_dir.is_some() || gl_username.is_some();

        //        println!("Current Path: {:?}, Current Git DIR: {:?}", env::current_exe().unwrap(), git_os_dir);

        if in_hook {
            let cwd = if let Some(git_os_path) = git_os_dir {
                PathBuf::from(git_os_path)
            } else {
                env::current_dir().unwrap()
            };
            let git_path = canonicalize(cwd).expect("Cannot open the GIT_DIR");
            if git_path.join("data").join(SETTINGS_FILE).is_file() {
                ExecEnv::Subgit(SubGitEnv {
                    git_dir: git_path.clone(),
                    hook_path: git_path.join("data").join("hook"),
                })
            } else {
                let hook_path = find_subgit_from_hook().expect("Cannot follow symlink");
                let repo_path = hook_path.parent().unwrap().parent().unwrap();
                if !repo_path.join("data").join(SETTINGS_FILE).is_file() {
                    panic!("Cannot find subgit path!");
                };
                ExecEnv::Upstream(SubGitEnv {
                    git_dir: repo_path.to_owned(),
                    hook_path: hook_path.to_owned(),
                })
            }
        } else {
            ExecEnv::Setup(env::current_exe().expect("Cannot read current executable path"))
        }
    }

    pub fn parse_command<I>(self, iterable: I) -> Result<Action, Box<Error>>
    where
        I: IntoIterator,
        I::Item: Into<OsString> + Clone,
    {
        match self {
            ExecEnv::Upstream(env) => {
                let mut s = String::new();
                std::io::stdin().read_to_string(&mut s).unwrap();

                let reqs = s
                    .lines()
                    .map(|v| v.trim())
                    .map(|line| -> Result<action::RefSyncRequest, Box<Error>> {
                        let entries = line.split(" ").collect::<Vec<&str>>();
                        match entries[..] {
                            [old_sha, new_sha, ref_name] => Ok(action::RefSyncRequest {
                                ref_name: ref_name.to_string(),
                                old_upstream_sha: Oid::from_str(old_sha)?,
                                new_upstream_sha: Oid::from_str(new_sha)?,
                            }),
                            _ => Err(Box::new(StringError {
                                message: "Bad args".to_owned(),
                            })),
                        }
                    })
                    .collect::<Result<Vec<action::RefSyncRequest>, Box<Error>>>()?;

                Ok(Action::SyncRefs(action::SyncRefs {
                    env,
                    requests: reqs,
                }))
            }
            ExecEnv::Subgit(env) => {
                let args: Vec<_> = iterable.into_iter().collect();
                let string_args: Vec<String> = args
                    .iter()
                    .map(|v| v.clone().into().to_string_lossy().into_owned())
                    .collect();
                string_args.iter().for_each(|v| debug!("Arg: {}", v));
                match args.len() {
                    2 => match string_args[1].as_str() {
                        "sync-all" => Ok(Action::SyncAll(action::SyncAll { env })),
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
