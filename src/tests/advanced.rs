use std::path::PathBuf;
use std::path::Path;
use std;
use util;
use std::error::Error;
use tests::test_util;
use WrappedSubGit;
use log::LevelFilter;
use model::BinSource;
use tests::test_util::clone;
use tests::test_util::set_credentials;
use std::time::Duration;
use std::thread::sleep;

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

        test_util::assert_dir_content_equal(&self.upstream.path.join(&self.upstream_sub_path), &self.downstream.path.join(&self.downstream_sub_path));
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
        let root = test_util::test_dir(name.as_ref());

        let d: &Path = root.as_ref();
        let up_bare = test_util::init_bare_repo("upstream.git", &d)?;
        let up = test_util::clone(&d, &up_bare)?;

        test_util::set_credentials(&up_bare);
        test_util::set_credentials(&up);

        let upstream = ExtGit { path: d.join("upstream") };

        setup(&upstream);

        let local_bare = test_util::init_bare_repo("subgit.git", &d)?;

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
}

struct Content {
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

    pub fn remove <P: AsRef<Path>>(self, path: P) -> Self{
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