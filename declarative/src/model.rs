use std::path::PathBuf;
use std::collections::HashMap;
use std::path::Path;


#[derive(Clone, Debug)]
pub enum FileChange {
    Deleted,
    Content(String)
}

#[derive(Debug, Clone)]
pub struct ChangeSet {
    files: HashMap<PathBuf, FileChange>,
}

#[derive(Copy, Clone, Debug)]
pub struct CommitRef(usize);

#[derive(Clone, Debug)]
pub struct Commit {
    message: String,
    changes: ChangeSet,
    parents: Vec<CommitRef>,
    id: CommitRef,
}

#[derive(Clone, Debug)]
pub struct CommitTree {
    commits: Vec<Commit>,
    branches: HashMap<String, CommitRef>,
}

pub struct TestSetup {
    path: PathBuf,
}

pub struct Test {
    path: PathBuf,
    subgit_base: String,
    paths: Vec<String>,
}

pub struct Consumer {
    name: String,
    path: PathBuf,
    base: String,
}

pub struct ChangeSetGenerator {
    subgit_base: PathBuf,
}

struct LabelBasedChangeSet {
    subgit_base: PathBuf,
    add_upstream: bool,
    add_subgit: bool,
}

pub trait ChangesForLabel {
    fn construct(self, label: &str) -> ChangeSet;
}



impl ChangeSet {
    pub fn add(&mut self, path: impl AsRef<Path>, change: FileChange) {
        self.files.insert(path.as_ref().to_owned(), change);
    }

    pub fn new() -> ChangeSet {
        ChangeSet {
            files: HashMap::new(),
        }
    }
}


impl ChangesForLabel for LabelBasedChangeSet {
    fn construct(self, label: &str) -> ChangeSet {
        let mut change_set = ChangeSet::new();
        if self.add_upstream {
            change_set.add(PathBuf::new().join(&format!("up-{}", &label)), FileChange::Content(format!("New content in upstream area.\nLabel: {}", &label)))
        }
        if self.add_subgit {
            change_set.add(self.subgit_base.join(&format!("sb-{}", &label)), FileChange::Content(format!("New content in subgit area.\nLabel: {}", &label)))
        }
        change_set
    }
}

impl ChangeSetGenerator {
    pub fn new(subgit_base: impl AsRef<Path>) -> ChangeSetGenerator {
        ChangeSetGenerator {
            subgit_base: subgit_base.as_ref().to_owned(),
        }
    }

    fn for_values(&self, add_upstream: bool, add_subgit: bool) -> impl ChangesForLabel {
        LabelBasedChangeSet {
            subgit_base: self.subgit_base.clone(),
            add_upstream,
            add_subgit,
        }
    }

    pub fn upstream(&self) -> impl ChangesForLabel {
        self.for_values(true, false)
    }
    pub fn subgit(&self) -> impl ChangesForLabel {
        self.for_values(false, true)
    }
    pub fn empty(&self) -> impl ChangesForLabel {
        self.for_values(false, false)
    }
    pub fn both(&self) -> impl ChangesForLabel {
        self.for_values(true, true)
    }
}


struct MergeCommit;

impl ChangesForLabel for MergeCommit {
    fn construct(self, label: &str) -> ChangeSet {
        ChangeSet::new()
    }
}

impl CommitTree {
    pub fn new() -> CommitTree {
        CommitTree {
            commits: Vec::new(),
            branches: HashMap::new(),
        }
    }

    fn add_commit(commits: &mut Vec<Commit>, label: &str, changes: impl ChangesForLabel, parents: Vec<CommitRef>) -> CommitRef {
        let commit_ref = CommitRef(commits.len());
        let commit = Commit {
            message: format!("Commit: {}", &label),
            changes: changes.construct(&label),
            parents: parents,
            id: CommitRef(commits.len()),
        };
        commits.push(commit);
        commit_ref
    }

    pub fn root(&mut self, label: &str, changes: impl ChangesForLabel) -> CommitRef {
        CommitTree::add_commit(&mut self.commits, label, changes, vec!())
    }

    pub fn commit(&mut self, label: &str, changes: impl ChangesForLabel, parent: CommitRef) -> CommitRef {
        CommitTree::add_commit(&mut self.commits, label, changes, vec!(parent))
    }

    pub fn merge_2(&mut self, label: &str, parent: CommitRef, other_parent: CommitRef) -> CommitRef {
        CommitTree::add_commit(&mut self.commits, label, MergeCommit, vec!(parent, other_parent))
    }

    pub fn branch(&mut self, name: &str, id: CommitRef){
        self.branches.insert(name.to_owned(), id);
    }
}