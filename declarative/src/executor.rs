use std::path::PathBuf;

use subgit_sync::SetupRequest;

use crate::harness::{TestConfig, Executor};
use std::process::ExitStatus;
use std::path::Path;
use log::LevelFilter;

pub struct DefaultExecutor {
    pub log_file: Option<String>,
    pub log_level: Option<LevelFilter>,
    pub use_daemon: bool,
}

const ROOT : &'static str = "test_data";
const SUBGIT: &'static str = "subgit.git";
const UPSTREAM: &'static str = "upstream.git";

impl Executor for DefaultExecutor {
    fn get_root(&self, harness: &TestConfig) -> PathBuf {
        crate::util::get_top().join(ROOT).join(&harness.module).join(&harness.test)
    }

    fn setup_args(&self, harness: &TestConfig) -> SetupRequest {
        let root = self.get_root(harness);
        SetupRequest {
            upstream_git_location: root.join(UPSTREAM).to_string_lossy().into(),
            subgit_git_location: root.join(SUBGIT).to_string_lossy().into(),
            upstream_map_path: "subgit".to_owned(),
            subgit_map_path: None,
            log_level: self.log_level.clone(),
            log_file: self.log_file.as_ref().map(|name| root.join(name)),
            upstream_hook_path: "hooks/post-receive".into(),
            subgit_hook_path: "hooks/update".into(),
            upstream_working_clone_url: None,
            subgit_working_clone_url: None,
            env_based_recursion_detection: None,
            disable_recursion_detection: true,
            match_ref:  "refs/heads/,HEAD".into(),
        }
    }
}

trait ToArgs {
    fn to_args(self) -> Vec<String>;
}

impl ToArgs for SetupRequest {
    fn to_args(self) -> Vec<String> {
        let mut base = vec!(
            self.upstream_git_location,
            self.subgit_git_location,
            self.upstream_map_path,
        );

        if let Some(subgit_map_path) = self.subgit_map_path {
            base.push("-p".to_owned());
            base.push(subgit_map_path);
        }

        if let Some(log_level) = self.log_level {
            base.push("-l".to_owned());
            base.push(format!("{}", log_level));
        }

        if let Some(log_file) = self.log_file {
            base.push("-f".to_owned());
            base.push(log_file.to_string_lossy().to_owned().to_string());
        }

        base.push("-H".to_owned());
        base.push(self.upstream_hook_path.to_string_lossy().to_owned().to_string());


        base.push("-h".to_owned());
        base.push(self.subgit_hook_path.to_string_lossy().to_owned().to_string());

        if let Some(upstream_working_clone_url) = self.upstream_working_clone_url {
            base.push("-U".to_owned());
            base.push(upstream_working_clone_url);
        }
        if let Some(subgit_working_clone_url) = self.subgit_working_clone_url {
            base.push("-u".to_owned());
            base.push(subgit_working_clone_url);
        }

        if let Some(env_based_recursion_detection) = self.env_based_recursion_detection {
            base.push("-r".to_owned());
            base.push(format!("{}:{}", env_based_recursion_detection.name, env_based_recursion_detection.value));
        }

        if self.disable_recursion_detection {
            base.push("-w".to_owned());
        }

        base.push("-m".to_owned());
        base.push(self.match_ref);

        base
    }
}

pub trait Runnable {
    fn run<CWD: AsRef<Path>>(self, cwd: CWD);
}

impl Runnable for SetupRequest {
    fn run<CWD: AsRef<Path>>(self, cwd: CWD) {
        let hook = crate::util::get_top().join("target/debug/subgit-sync");
        eprintln!("{:?}", hook);
        let mut process = std::process::Command::new(hook);
        process
            .env_clear()
            .env("PATH", std::env::var("PATH").unwrap());
        process.current_dir(cwd.as_ref());
        for v in self.to_args() {
            process.arg(v);
        }
        process.env("RUST_BACKTRACE", "1");
        let res: ExitStatus = process.spawn().expect("Could not launch setup process").wait().expect("Could not complete setup process");
        assert_eq!(true, res.success())
    }
}