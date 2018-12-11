

use std::cmp::Ord;
use std::path::PathBuf;
use std::error::Error;
use std::str::FromStr;
use std::num::ParseIntError;
use std::cmp::Ordering;
use crate::tree::Commit;
use crate::tree::FileChange;
use std::path::Path;
use crate::tree::ChangeSet;

const NEEDS_ALLOW_UNRELATED_HISTORIES_MERGE_FLAG: Version = Version {
    major: 2,
    minor: 9,
    dot: 0,
};


#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct Version {
    major: u32,
    minor: u32,
    dot: u32,
}

#[derive(Debug, Clone)]
pub struct Git {
    path: PathBuf,
    git_version: Version,
}

pub type GitResult = Result<(), Box<Error>>;

fn get_git_version() -> Result<Version, Box<Error>> {
    let version_string = String::from_utf8(
        crate::util::command_raw(std::env::current_dir()?, "git", vec!["--version"].iter())?.stdout,
    )?;
    Ok(version_string.split(" ").last().unwrap().parse()?)
}

/* Version stuff */

impl FromStr for Version {
    type Err = ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let coords: Vec<&str> = s.split('.').collect();

        let major = coords[0].parse::<u32>()?;
        let minor = coords[1].parse::<u32>()?;
        let dot = coords[1].parse::<u32>()?;

        Ok(Version { major, minor, dot })
    }
}

impl Ord for Version {
    fn cmp(&self, other: &Version) -> Ordering {
        if self.major.cmp(&other.major) == Ordering::Equal {
            if self.minor.cmp(&other.minor) == Ordering::Equal {
                self.dot.cmp(&other.dot)
            } else {
                self.minor.cmp(&other.minor)
            }
        } else {
            self.major.cmp(&other.major)
        }
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Version) -> Option<Ordering> {
        Some(self.cmp(&other))
    }
}

pub struct GitWrapper {
    git: Git
}

impl FileChange {
    pub fn apply<R: AsRef<Path>, P: AsRef<Path>>(&self, root: R, path: P) {
        match self {
            FileChange::Deleted => std::fs::remove_file(&root.as_ref().join(path.as_ref())).unwrap(),
            FileChange::Content(content)=> {
                std::fs::create_dir_all(&root.as_ref().join(path.as_ref()).parent().unwrap())
                    .unwrap();
                std::fs::write(&root.as_ref().join(path.as_ref()), &content).unwrap()
            }
        }
    }
}

pub trait InternalGit {
    fn commit_orphan(&self, message: String, change_set: ChangeSet) -> String;
    fn commit_merge(&self, message: String, parents: Vec<String>) -> String;
    fn commit_child(&self, message: String, change_set: ChangeSet, parent: String) -> String;
    fn make_branch(&self, branch: String, commit: String);
    fn push_all(&self);
    fn set_head(&self, branch: String);
}

impl InternalGit for GitWrapper {
    fn commit_orphan(&self, message: String, change_set: ChangeSet) -> String {
        self.git.command(&["checkout", "--orphan", "O_R_P_H_A_N"]).unwrap();
        self.git.command(&["rm", "-rf", "."]).unwrap_or({}); // It's okay if it fails - it fails if there are no files
        for (path, action) in change_set.files {
            action.apply(&self.git.path, path);
        }
        self.git.command(&["add", "."]).unwrap();
        self.git.command(&["commit", "--allow-empty", "-m", &message]).expect(&format!("failed to commit: {}", &message));
        let sha = self.git.get_current_commit().unwrap();
        self.git.checkout(&sha).expect(&format!("Could not checkout parent for '{}' (parent: {})", &message, &sha));
        self.git.command(&["branch", "-D", "O_R_P_H_A_N"]).expect("Failed to delete orphan branch");
        sha
    }

    fn commit_merge(&self, message: String, parents: Vec<String>) -> String {
        let mut parents = parents;
        let first = parents.remove(0);
        self.checkout_commit(first);
        let mut command_parts = vec!("merge", "-m", &message, "--no-edit");
        let mut rest : Vec<&str> = parents.iter().map(|v| v.as_ref()).collect();
        command_parts.append(&mut rest);
        self.git.command(&command_parts).unwrap();
        self.git.get_current_commit().unwrap()
    }

    fn commit_child(&self, message: String, change_set: ChangeSet, parent: String) -> String {
        self.checkout_commit(parent);
        for (path, action) in change_set.files {
            action.apply(&self.git.path, path);
        }
        self.git.command(&["add", "."]).expect("Failed to add files");
        self.git.command(&["commit", "--allow-empty", "-m", &message]).expect("Couldn't make commit!");
        self.git.get_current_commit().expect("Couldn't get current commit")
    }

    fn make_branch(&self, branch: String, commit: String) {
        self.checkout_commit(commit);
        self.git.branch(["-f", &branch, "HEAD"]).unwrap()
    }

    fn push_all(&self){
        self.git.command(&["push", "--all"]).unwrap();
    }

    fn set_head(&self, branch: String) {
        self.git.command(&["remote","set-head","origin", &branch]).unwrap()
    }
}

// Rework these to take into account the subgit path
// Maybe separate this out for code reuse - e.g. commit vs not
// Or maybe easy delegating
impl GitWrapper {
    pub fn new(path: PathBuf) -> GitWrapper {
        GitWrapper {
            git: Git {
                path,
                git_version: get_git_version().unwrap(),
            }
        }
    }

    fn checkout_commit(&self, sha: String) -> &Self {
        #![allow(unused)]
        self.git.command(&["checkout", &sha]).unwrap();
        self
    }
}

pub struct Consumer {
    /// Filter for the subgit
    filter: Option<String>,
    git: Git,
}

impl Consumer {
    pub fn new(path: PathBuf, filter: Option<String>) -> Consumer {
        Consumer {
            filter,
            git: Git {
                path,
                git_version: get_git_version().unwrap(),
            }
        }
    }
}

pub trait GitConsumer {
    fn commit(&self, commit: Commit) -> (String, &Self);
    fn merge(&self, commit: Commit, other_sha: &str) -> (String, &Self);
    fn ff(&self, other_sha: &str) -> (String, &Self);

    fn pull(&self) -> &Self;
    fn pull_ff_only(&self) -> &Self;
    fn pull_merge(&self) -> &Self;
    fn pull_conflict(&self) -> &Self;

    fn push(&self, branch_name: &str) -> &Self;
    fn push_fail(&self, branch_name: &str) -> &Self;
    fn push_all(&self) -> &Self;

    fn checkout_commit(&self, sha: &str) -> &Self;
    fn checkout_branch(&self, branch: &str) -> &Self;
}

impl <T: GitConsumer> GitConsumer for (String, &T) {
    fn commit(&self, commit: Commit) -> (String, &Self) {
        (self.1.commit(commit).0, self)
    }

    fn merge(&self, commit: Commit, other_sha: &str) -> (String, &Self) {
        (self.1.merge(commit, other_sha).0, self)
    }

    fn ff(&self, other_sha: &str) -> (String, &Self) {
        (self.1.ff(other_sha).0, self)
    }

    fn pull(&self) -> &Self {
        self.1.pull(); self
    }

    fn pull_ff_only(&self) -> &Self {
        self.1.pull_ff_only(); self
    }

    fn pull_merge(&self) -> &Self {
        self.1.pull_merge(); self
    }

    fn pull_conflict(&self) -> &Self {
        self.1.pull_conflict(); self
    }

    fn push(&self, branch_name: &str) -> &Self {
        self.1.push(branch_name); self
    }

    fn push_fail(&self, branch_name: &str) -> &Self {
        self.1.push_fail(branch_name); self
    }

    fn push_all(&self) -> &Self {
        self.1.push_all(); self
    }

    fn checkout_commit(&self, sha: &str) -> &Self {
        self.1.checkout_commit(sha); self
    }

    fn checkout_branch(&self, branch: &str) -> &Self {
        self.1.checkout_commit(branch); self
    }
}

impl GitConsumer for Consumer {
    fn commit(&self, commit: Commit) -> (String, &Self) {
        eprintln!("{} - {:?}", &commit.message, &self.filter);
        for (path, action) in commit.changes.files {
            if let Some(ref value) = self.filter {
                let path_string = path.to_string_lossy().to_owned();
                let path = (&path_string).replace(value, "");

                action.apply(&self.git.path, path);
            } else {
                action.apply(&self.git.path, path);
            }
        }
        self.git.command(&["add", "."]).expect("Unable to add files for commit");
        self.git.commit(&commit.message).expect(&format!("Couldn't make commit! : {}\n", &commit.message));
        (self.git.get_current_commit().expect("Couldn't get current commit"), self)
    }

    fn merge(&self, commit: Commit, other_sha: &str) -> (String, &Self) {
        self.git.command(&["merge", "-m", &commit.message, "--no-edit", other_sha]).unwrap();
        (self.git.get_current_commit().expect("Couldn't get current commit"), self)
    }

    fn ff(&self, other_sha: &str) -> (String, &Self) {
        self.git.command(&["merge", "--ff-only", other_sha]).unwrap();
        (self.git.get_current_commit().expect("Couldn't get current commit"), self)
    }

    fn pull(&self) -> &Self {
        self.git.pull().unwrap();
        self
    }

    fn pull_ff_only(&self) -> &Self {
        self.git.command(&["pull", "--ff-only"]).unwrap();
        self
    }

    fn pull_merge(&self) -> &Self {
        self.pull()
    }

    fn pull_conflict(&self) -> &Self {
        unimplemented!()
    }

    fn push(&self, branch: &str) -> &Self {
        self.git.command(&["push", "-u", "origin", branch]).unwrap();
        self
    }

    fn push_fail(&self, branch: &str) -> &Self {
        assert_eq!(true,self.git.command(&["push", "-u", "origin", branch]).is_err());
        self
    }

    fn push_all(&self) -> &Self {
        self.git.command(&["push", "--all"]).unwrap();
        self
    }

    fn checkout_commit(&self, sha: &str) -> &Self {
        self.git.command(&["checkout", &sha]).unwrap();
        self
    }

    fn checkout_branch(&self, branch: &str) -> &Self {
        self.git.command(&["checkout", "-b", &branch]).unwrap();
        self
    }
}

/* Basic git stuff */
#[allow(unused)]
impl Git {
    pub fn path(&self) -> PathBuf {
        self.path.clone()
    }

    fn command(&self, args: &[&str]) -> GitResult {
        crate::util::command(&self.path, "git", args.iter())
    }

    pub fn commit<M: AsRef<str>>(&self, message: M) -> GitResult {
        self.command(&["commit", "-m", message.as_ref()])
    }

    pub fn add<P: AsRef<str>>(&self, path: P) -> GitResult {
        self.command(&["add", "--all", path.as_ref()])
    }

    pub fn checkout<R: AsRef<str>>(&self, ref_ish: R) -> GitResult {
        self.command(&["checkout", ref_ish.as_ref()])
    }

    pub fn pull(&self) -> GitResult {
        self.command(&["pull"])
    }

    pub fn push(&self) -> GitResult {
       self.command(&["push"])
    }

    pub fn branch<A: AsRef<str>, P: AsRef<[A]>>(&self, args: P) -> GitResult {
        let mut args: Vec<&str> = args.as_ref().into_iter().map(|v| v.as_ref()).collect();
        args.insert(0, "branch");
        self.command(&args)
    }

    pub fn checkout_adv<A: AsRef<str>, P: AsRef<[A]>>(&self, args: P) -> GitResult {
        let mut args: Vec<&str> = args.as_ref().into_iter().map(|v| v.as_ref()).collect();
        args.insert(0, "checkout");
        self.command(&args)
    }

    pub fn push_adv<A: AsRef<str>, P: AsRef<[A]>>(&self, args: P) -> GitResult {
        let mut args: Vec<&str> = args.as_ref().into_iter().map(|v| v.as_ref()).collect();
        args.insert(0, "push");
        self.command(&args)
    }

    pub fn merge<A: AsRef<str>, P: AsRef<[A]>>(&self, args: P) -> GitResult {
        let mut args: Vec<&str> = args.as_ref().into_iter().map(|v| v.as_ref()).collect();
        args.insert(0, "merge");
        if self.git_version > NEEDS_ALLOW_UNRELATED_HISTORIES_MERGE_FLAG {
            args.insert(1, "--allow-unrelated-histories");
        }
        self.command(&args)
    }

    pub fn commit_count<S: AsRef<str>>(&self, commit_ish: S) -> Result<u32, Box<Error>> {
        let args = vec!["rev-list", "--count", &commit_ish.as_ref()];
        let command_output = self.command_output(args)?;
        //Ok( String::from_utf8(command_res.stdout).unwrap())?.parse().unwrap())
        Ok(command_output.trim().parse()?)
    }

    pub fn get_current_commit(&self) -> Result<String, Box<Error>> {
        let args = vec!["rev-parse", "HEAD"];
        self.command_output(args).map(|v| v.trim().to_owned())
    }

    pub fn command_output(&self, args: Vec<&str>) -> Result<String, Box<Error>> {
        let command_res = crate::util::command_raw(&self.path, "git", args.iter())?;
        let res_out = String::from_utf8((command_res).stdout).unwrap();
        Ok(res_out.trim().to_owned())
    }
}