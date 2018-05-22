use std::path::PathBuf;
use std::path::Path;
use std;
use std::error::Error;
use subgit_rs::{WrappedSubGit, BinSource};
use log::LevelFilter;
use std::time::Duration;
use std::thread::sleep;
use simplelog::TermLogger;
use simplelog::Config;
use super::util::*;
use super::util;
use std::thread;

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

pub struct ExtGit {
    path: PathBuf,
}

pub enum GitType {
    Upstream,
    Subgit,
}

impl TestWrapper {
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

    pub fn new<P : AsRef<str>, S: FnOnce(&ExtGit) -> ()>(name: P, setup: S, subgit_eq: &str) -> Result<TestWrapper, Box<Error>> {
        let root = test_dir(name.as_ref());

        let d: &Path = root.as_ref();
        let up_bare = init_bare_repo("upstream.git", &d)?;
        let up = clone(&d, &up_bare)?;

        set_credentials(&up_bare);
        set_credentials(&up);

        let upstream = ExtGit { path: d.join("upstream") };

        setup(&upstream);

        let local_bare = init_bare_repo("subgit.git", &d)?;

        let wrapped = WrappedSubGit::run_creation(
            &local_bare,
            &up_bare,
            "sub",
            None,
            LevelFilter::Debug,
            BinSource {
                location: PathBuf::from("target/debug/hook"),
                symlink: true,
            },
            None,
            None,
        )?;

        wrapped.update_all_from_upstream()?;

        let local = clone(&d, &local_bare)?;

        set_credentials(&local_bare);
        set_credentials(&local);

        let wrapper = TestWrapper {
            root: root.clone(),
            upstream,
            downstream: ExtGit { path: d.join("subgit") },
            upstream_sub_path: PathBuf::from(subgit_eq),
            downstream_sub_path: PathBuf::from(""),
        };

        wrapper.do_then_verify(|_,_| {Ok(())});

        Ok(wrapper)
    }
}

impl ExtGit {
    pub fn update_working(&self, files: Vec<FileAction>){
        files.iter().for_each(|file_action| file_action.apply(&self.path));
    }

    pub fn commit<M: AsRef<str>>(&self, message: M) -> GitResult {
        util::command(&self.path, "git", ["commit", "-m", message.as_ref()].iter())
    }

    pub fn add<P: AsRef<str>>(&self, path: P) -> GitResult {
        util::command(&self.path, "git", ["add", path.as_ref()].iter())
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

    pub fn commit_count<S: AsRef<str>>(&self, commit_ish: S) -> GitResult {
        let mut args = vec!["rev-list", "--count", &commit_ish.as_ref()];
        util::command(&self.path, "git", args.iter())
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

    {
        util::write_files(&up, files.next().unwrap())?;
        util::command(&up, "git", ["add", "."].iter())?;
        util::command(&up, "git", ["commit", "-m", "(1) First upstream commit"].iter())?;
        util::command(&up, "git", ["push"].iter())?;
    };
    let local_bare = init_bare_repo("local.git", &d)?;
    let wrapped = WrappedSubGit::run_creation(
        &local_bare,
        &up_bare,
        "sub",
        None,
        LevelFilter::Debug,
        BinSource {
            location: PathBuf::from("target/debug/hook"),
            symlink: true,
        },
        None,
        None,
    )?;

    wrapped.update_all_from_upstream()?;

    let local = clone(&d, &local_bare)?;

    set_credentials(&local_bare);
    set_credentials(&local);

    assert_dir_content_equal(&local, &up.join("sub"));

    {
        util::write_files(&local, files.next().unwrap())?;

        util::command(&local, "git", ["add", "."].iter())?;
        util::command(&local, "git", ["commit", "-m", "(2) First subgit commit"].iter())?;
        util::command(&local, "git", ["push"].iter())?;

        util::command(&up, "git", ["pull"].iter())?;
    };

    assert_dir_content_equal(&local, &up.join("sub"));

    {
        util::write_files(&up, files.next().unwrap())?;

        util::command(&up, "git", ["add", "."].iter())?;
        util::command(&up, "git", ["commit", "-m", "(3) Second upstream commit"].iter())?;
        util::command(&up, "git", ["push"].iter())?;

        thread::sleep(Duration::new(3,0));

        util::command(&local, "git", ["pull"].iter())?;
    };

    assert_dir_content_equal(&local, &up.join("sub"));

    Ok(())
}

