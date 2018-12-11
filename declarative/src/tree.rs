use std::path::PathBuf;
use std::collections::HashMap;
use std::path::Path;
use std::prelude::v1::Vec;


#[derive(Clone, Debug)]
pub enum FileChange {
    Deleted,
    Content(String)
}

#[derive(Debug, Clone)]
pub struct ChangeSet {
    pub files: HashMap<PathBuf, FileChange>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct CommitRef(usize);

#[derive(Clone, Debug)]
pub struct Commit {
    pub message: String,
    pub changes: ChangeSet,
    parents: Vec<CommitRef>,
    id: CommitRef,
}

#[derive(Clone, Debug)]
pub struct CommitTree {
    commits: Vec<Commit>,
    branches: HashMap<String, CommitRef>,
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

pub struct TreeRecord {
    to_sha: HashMap<CommitRef, String>,
    to_branch: HashMap<CommitRef, String>,
}

impl TreeRecord {
    pub fn new() -> TreeRecord {
        TreeRecord {
            to_sha: HashMap::new(),
            to_branch: HashMap::new(),
        }
    }
}

impl ChangesForLabel for MergeCommit {
    fn construct(self, _label: &str) -> ChangeSet {
        ChangeSet::new()
    }
}

pub type FileContent = String;
pub type BranchName = String;

pub trait NiceGit {
    fn commit_orphan(&self, message: String, change_set: ChangeSet) -> String;
    fn commit_merge(&self, message: String, parents: Vec<String>) -> String;
    fn commit_child(&self, message: String, change_set: ChangeSet, parent: String) -> String;
    fn make_branch(&self, branch: String, commit: String);
}

impl CommitTree {
    pub fn new() -> CommitTree {
        CommitTree {
            commits: Vec::new(),
            branches: HashMap::new(),
        }
    }

    fn add_commit(commits: &mut Vec<Commit>, label: &str, changes: impl ChangesForLabel, parents: Vec<CommitRef>) -> CommitRef {
        assert_eq!(true, commits.len() as i32 > parents.iter().map(|v| v.0 as i32).max().unwrap_or(-1));
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

    pub fn commit_tree<GIT: NiceGit>(self, git: &GIT, record: &mut TreeRecord) {
        for mut commit in self.commits {
            eprintln!("Working on {}", &commit.message);
            if !record.to_sha.contains_key(&commit.id) {
                let sha = match commit.parents.len() {
                    0 => git.commit_orphan(commit.message, commit.changes),
                    1 => git.commit_child(commit.message, commit.changes, record.to_sha.get(&commit.parents.remove(0)).unwrap().to_owned()),
                    _n => git.commit_merge(commit.message, commit.parents.into_iter().map(|v| record.to_sha.get(&v).unwrap().to_owned()).collect()),
                };
                record.to_sha.insert(commit.id, sha);
            }
        }
        for (branch_name, commit_ref) in self.branches {
            let sha =  record.to_sha.get(&commit_ref).expect("SHA for branch should exist!").clone();
            git.make_branch(branch_name.clone(), sha);
            record.to_branch.insert(commit_ref, branch_name);
        }
    }

    pub fn reify(&self) -> HashMap<BranchName, HashMap<PathBuf, FileContent>> {
        self.branches.iter().map(|(branch, commit_ref)| {
            let branch = branch.clone();
            let ancestor_path = self.ancestor_path(*commit_ref);
            let mut file_map : HashMap<PathBuf, FileContent> = HashMap::new();

            ancestor_path.iter()
                .map(|v| &self.commits[v.0])
                .for_each(|commit| {
                    commit.changes.files.iter().for_each(|(path, action)| {
                        match action {
                            FileChange::Deleted => file_map.remove(path),
                            FileChange::Content(content) => file_map.insert(path.clone(), content.clone())
                        };
                    })
                });

                (branch, file_map)
        }).collect()
    }

    fn ancestor_path(&self, commit_ref: CommitRef) -> Vec<CommitRef> {
        let mut queue = vec!(commit_ref);
        let mut i = 0;
        while i < queue.len() {
            let item_ref = queue[i];
            let mut more_items = self.commits[item_ref.0].parents.clone();
            queue.append(&mut more_items);
            i = i + 1;
        }
        queue
    }
}