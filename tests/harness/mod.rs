use std::path::PathBuf;
use std::path::Path;
use std;
use std::error::Error;
use log::LevelFilter;
use std::time::Duration;
use std::thread::sleep;
use simplelog::TermLogger;
use simplelog::Config;
use super::util::*;
use super::util;
use std::thread;
use std::process::ExitStatus;

// Setup with list of initial commits (generate predictable commit messages if none are given)
// Do changes & commit
// Pull
// Push

pub type GitResult = Result<(), Box<Error>>;

pub struct TestWrapper {
    root: PathBuf,
    upstream: ExtGit,
    downstream: ExtGit,
    upstream_sub_path: PathBuf,
    downstream_sub_path: PathBuf,
}

#[derive(Debug, Clone)]
pub struct ExtGit {
    path: PathBuf,
}

pub enum GitType {
    Upstream,
    Subgit,
}

impl TestWrapper {
    pub fn get_subgit(&self) -> ExtGit{
        self.downstream.clone()
    }
    pub fn get_upstream(&self) -> ExtGit{
        self.upstream.clone()
    }

    pub fn do_then_verify<D: FnOnce(&ExtGit, &ExtGit) -> Result<(), Box<Error>>>(&self, doer: D) {
        let res = doer(&self.upstream, &self.downstream);
        res.unwrap();

        assert_dir_content_equal(&self.upstream.path.join(&self.upstream_sub_path), &self.downstream.path.join(&self.downstream_sub_path));
    }

    pub fn verify_push_changes_and_pull_in_other(&self, main: GitType, changes: Vec<FileAction>, message: &str){
        let source = match &main {
            &GitType::Upstream => &self.upstream,
            &GitType::Subgit => &self.downstream,
        };
        let dest = match &main {
            &GitType::Upstream => &self.downstream,
            &GitType::Subgit => &self.upstream,
        };
        let delay = match &main {
            &GitType::Upstream => Some(Duration::new(3,0)),
            &GitType::Subgit => None,
        };

        source.update_working(changes);
        source.add(".").unwrap();
        source.commit(message).unwrap();
        source.push().unwrap();

        delay.map(|dur| sleep(dur));

        dest.pull().unwrap();
    }

    fn new_instance<P : AsRef<Path>, S: FnOnce(&ExtGit) -> ()>(root: P, setup: S, subgit_eq: &str, extra: &[&str]) -> Result<TestWrapper, Box<Error>> {
        let d: &Path = root.as_ref();
        let up_bare = init_bare_repo("upstream.git", &d)?;
        let up = clone(&d, &up_bare)?;

        set_credentials(&up_bare);
        set_credentials(&up);
        set_push_setting(&up);

        let upstream = ExtGit { path: d.join("upstream") };

        setup(&upstream);

        let local_bare = init_bare_repo("subgit.git", &d)?;

        // Run the setup...
        let mut process = std::process::Command::new("target/debug/subgit-rs");
        process
            .env_clear()
            .env("PATH", std::env::var("PATH").unwrap());
        process.arg(&up_bare.as_path().to_string_lossy().as_ref());
        process.arg(&local_bare.as_path().to_string_lossy().as_ref());
        for v in  extra {
            process.arg(*v);
        }
        process.arg(subgit_eq);
        process.env("RUST_BACKTRACE", "1");

        let res : ExitStatus = process.spawn().unwrap().wait().unwrap();
        assert!(res.success());

        let local = clone(&d, &local_bare)?;

        set_credentials(&local_bare);
        set_credentials(&local);
        set_push_setting(&local);

        let wrapper = TestWrapper {
            root: root.as_ref().to_owned(),
            upstream,
            downstream: ExtGit { path: d.join("subgit") },
            upstream_sub_path: PathBuf::from(subgit_eq),
            downstream_sub_path: PathBuf::from(""),
        };

        wrapper.do_then_verify(|_,_| {Ok(())});

        Ok(wrapper)
    }

    pub fn new_adv<P : AsRef<str>, S: FnOnce(&ExtGit) -> (), A: FnOnce(&Path) -> Vec<String>>(name: P, setup: S, subgit_eq: &str, gen_args: A) -> Result<TestWrapper, Box<Error>> {
        let root = test_dir(name.as_ref());
        let extra = gen_args(&root);
        let temp_log_path = root.clone().join("test_setup.log");
        let log_path = temp_log_path.to_string_lossy();
        let mut extra_args = vec!("-f", &log_path);
        extra.iter().for_each(|v| extra_args.push(v));
        TestWrapper::new_instance(&root, setup, subgit_eq, &extra_args)
    }

    pub fn new<P : AsRef<str>, S: FnOnce(&ExtGit) -> ()>(name: P, setup: S, subgit_eq: &str) -> Result<TestWrapper, Box<Error>> {
        TestWrapper::new_adv(name, setup, subgit_eq, |r| vec!())
    }
}


fn set_push_setting<P: AsRef<Path>>(path: P) {
    util::command(path, "git", ["config", "push.default", "simple"].iter()).unwrap();
}

impl ExtGit {
    pub fn update_working(&self, files: Vec<FileAction>){
        files.iter().for_each(|file_action| file_action.apply(&self.path));
    }

    pub fn commit<M: AsRef<str>>(&self, message: M) -> GitResult {
        util::command(&self.path, "git", ["commit", "-m", message.as_ref()].iter())
    }

    pub fn add<P: AsRef<str>>(&self, path: P) -> GitResult {
        util::command(&self.path, "git", ["add", "--all", path.as_ref()].iter())
    }

    pub fn checkout<R: AsRef<str>>(&self, ref_ish: R) -> GitResult {
        util::command(&self.path, "git", ["checkout", ref_ish.as_ref()].iter())
    }

    pub fn pull(&self) -> GitResult {
        util::command(&self.path, "git", ["pull"].iter())
    }

    pub fn push(&self) -> GitResult {
        util::command(&self.path, "git", ["push"].iter())
    }

    pub fn branch<A: AsRef<str>, P: AsRef<[A]>>(&self, args: P) -> GitResult {
        let mut args : Vec<&str> = args.as_ref().into_iter().map(|v| v.as_ref()).collect();
        args.insert(0, "branch");
        util::command(&self.path, "git", args.iter())
    }

    pub fn checkout_adv<A: AsRef<str>, P: AsRef<[A]>>(&self, args: P) -> GitResult {
        let mut args : Vec<&str> = args.as_ref().into_iter().map(|v| v.as_ref()).collect();
        args.insert(0, "checkout");
        util::command(&self.path, "git", args.iter())
    }

    pub fn push_adv<A: AsRef<str>, P: AsRef<[A]>>(&self, args: P) -> GitResult {
        let mut args : Vec<&str> = args.as_ref().into_iter().map(|v| v.as_ref()).collect();
        args.insert(0, "push");
        util::command(&self.path, "git", args.iter())
    }

    pub fn merge<A: AsRef<str>, P: AsRef<[A]>>(&self, args: P) -> GitResult {
        let mut args : Vec<&str> = args.as_ref().into_iter().map(|v| v.as_ref()).collect();
        args.insert(0, "merge");
        util::command(&self.path, "git", args.iter())
    }

    pub fn commit_count<S: AsRef<str>>(&self, commit_ish: S) -> Result<u32, Box<Error>> {
        let mut args = vec!["rev-list", "--count", &commit_ish.as_ref()];
        let command_res = util::command_raw(&self.path, "git", args.iter())?;
        let res_out = String::from_utf8((command_res).stdout).unwrap();
        println!("--{}--", res_out);
        //Ok( String::from_utf8(command_res.stdout).unwrap())?.parse().unwrap())
        Ok(res_out.trim().parse()?)
    }

    pub fn command_output(&self, args: Vec<&str>) -> Result<String, Box<Error>> {
        let command_res = util::command_raw(&self.path, "git", args.iter())?;
        let res_out = String::from_utf8((command_res).stdout).unwrap();
        Ok(res_out.trim().to_owned())
    }
}

pub struct Content {
    path: PathBuf,
    content: Vec<u8>,
}

pub enum FileAction {
    Remove(PathBuf),
    Assign(Content),
}

impl FileAction{
    pub fn overwrite <P: AsRef<Path>, C: AsRef<[u8]>>(path: P, content: C) -> FileAction{
        FileAction::Assign(Content {
            path: path.as_ref().to_owned(),
            content: content.as_ref().iter().map(|v| *v).collect()
        })
    }

    pub fn remove <P: AsRef<Path>>(path: P) -> Self{
        FileAction::Remove(path.as_ref().to_owned())
    }

    fn apply<R: AsRef<Path>>(&self, root: R) {
        match self {
            FileAction::Remove(path) => std::fs::remove_file(&root.as_ref().join(path)).unwrap(),
            FileAction::Assign(content) => {
                std::fs::create_dir_all(&root.as_ref().join(&content.path).parent().unwrap()).unwrap();
                std::fs::write(&root.as_ref().join(&content.path), &content.content).unwrap()
            },
        }
    }
}


/* Simpler test harness */
pub fn run_basic_branch_test<P,K,V,F,I>(root: P, files_collection: I) -> Result<(), Box<Error>>
where P: AsRef<Path>, K: AsRef<Path>, V: AsRef<[u8]>, F: IntoIterator<Item=(K,V)>, I: IntoIterator<Item=F>
{
    let mut files = files_collection.into_iter();

    let _ = TermLogger::init(LevelFilter::Debug, Config::default());
    let d = root.as_ref();
    let up_bare = init_bare_repo("test.git", &d)?;
    let up = clone(&d, &up_bare)?;

    set_credentials(&up_bare);
    set_credentials(&up);
    set_push_setting(&up);

    {
        util::write_files(&up, files.next().unwrap())?;
        util::command(&up, "git", ["add", "--all", "."].iter())?;
        util::command(&up, "git", ["commit", "-m", "(1) First upstream commit"].iter())?;
        util::command(&up, "git", ["push"].iter())?;
    };
    let local_bare = init_bare_repo("local.git", &d)?;

    // Run the setup...
    let mut process = std::process::Command::new("target/debug/subgit-rs");
    process
        .env_clear()
        .env("PATH", std::env::var("PATH").unwrap());
    process.arg(&up_bare.as_path().to_string_lossy().as_ref());
    process.arg(&local_bare.as_path().to_string_lossy().as_ref());
    process.arg("-f");
    process.arg(&d.join("test_setup.log"));
    process.arg("sub");
    process.env("RUST_BACKTRACE", "1");

    let res = process.spawn().unwrap().wait_with_output().unwrap();
    if !res.status.success() {
        println!("Setup - STD OUT:\n{}", String::from_utf8(res.stdout).unwrap());
        println!("Setup - STD ERR:\n{}", String::from_utf8(res.stderr).unwrap());
        assert!(false);
    }

    let local = clone(&d, &local_bare)?;

    set_credentials(&local_bare);
    set_credentials(&local);
    set_push_setting(&local);

    assert_dir_content_equal(&local, &up.join("sub"));

    {
        util::write_files(&local, files.next().unwrap())?;

        util::command(&local, "git", ["add", "--all", "."].iter())?;
        util::command(&local, "git", ["commit", "-m", "(2) First subgit commit"].iter())?;
        util::command(&local, "git", ["push"].iter())?;

        util::command(&up, "git", ["pull"].iter())?;
    };

    assert_dir_content_equal(&local, &up.join("sub"));

    {
        util::write_files(&up, files.next().unwrap())?;

        util::command(&up, "git", ["add", "--all", "."].iter())?;
        util::command(&up, "git", ["commit", "-m", "(3) Second upstream commit"].iter())?;
        util::command(&up, "git", ["push"].iter())?;

        thread::sleep(Duration::new(3,0));

        util::command(&local, "git", ["pull"].iter())?;
    };

    assert_dir_content_equal(&local, &up.join("sub"));

    Ok(())
}

