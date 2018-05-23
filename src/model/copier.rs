use git2::{Commit, Delta, Index, IndexAddOption, ObjectType, Oid, Repository, build::CheckoutBuilder};
use super::map::CommitMapper;
use std::path::{Path, PathBuf};
use std::fs::OpenOptions;
use std::io::Write;
use std::fs;
use git;

pub struct GitLocation<'a> {
    pub location: &'a Path,
    pub name: &'static str,
    pub bare: &'a Repository,
    pub working: &'a Repository,
}

pub struct Copier<'a> {
    pub source: GitLocation<'a>,
    pub dest: GitLocation<'a>,
    pub mapper: &'a CommitMapper<'a>,
}

impl<'a> GitLocation<'a> {
    fn working_index(&self) -> Index {
        self.working.index().unwrap()
    }

    fn workdir(&self) -> &Path {
        &self.working
            .workdir()
            .expect("The map repo must have a workdir")
    }

    pub fn get_commits_between(
        &self,
        maybe_starting_sha: Option<Oid>,
        dest_sha_inclusive: &Oid,
    ) -> Vec<Oid> {
        debug!("Finding commits in {} between {:?} and {:?}", self.bare.path().to_string_lossy(), maybe_starting_sha, dest_sha_inclusive);
        let walker = &mut self.bare.revwalk().unwrap();
        if let Some(starting_sha) = maybe_starting_sha {
            walker.hide(starting_sha).unwrap();
        }
        walker.push(*dest_sha_inclusive).unwrap();
        walker.set_sorting(git::reverse_topological());
        let res: Result<Vec<Oid>, _> = walker.collect();
        return res.unwrap();
    }
}

// Removes duplicates from the end of the vec, preserving the first seen order
// Unoptimized, but probably good enough for small vecs
fn dedup_vec<Item: Eq>(items: &mut Vec<Item>) {
    if items.len() < 2 {
        return;
    }
    let to_drop: Vec<_> = items
        .iter()
        .enumerate()
        .filter_map(|(idx, item)| {
            if items
                .iter()
                .skip(idx + 1)
                .any(|other_item| other_item == item)
            {
                Some(idx)
            } else {
                None
            }
        })
        .collect();
    to_drop.iter().rev().for_each(|idx| {
        items.remove(*idx);
    });
}

impl<'a> Copier<'a> {
    fn translate(&'a self, path: &Path) -> Option<PathBuf> {
        let chopped = path.strip_prefix(self.source.location).ok();
        chopped.map(|new_path| self.dest.workdir().join(self.dest.location.join(new_path)))
    }

    fn record_sha_update(&'a self, source_sha: &Oid, dest_sha: Oid) -> Oid {
        info!("Mapping {:?} <-> {:?}", source_sha, dest_sha);
        self.mapper
            .set_translated(source_sha, self.source.name, self.dest.name, &dest_sha);
        self.mapper
            .set_translated(&dest_sha, self.dest.name, self.source.name, source_sha);
        dest_sha
    }

    fn empty_sha(&'a self) -> Oid {
        self.dest
            .working
            .find_reference("refs/sync/empty")
            .expect("An empty commit should exist")
            .target()
            .expect("An empty commit should be an oid, not another reference")
    }

    pub fn get_dest_sha(&'a self, source_sha: &Oid) -> Oid {
        self.mapper
            .get_translated(Some(*source_sha), self.source.name, self.dest.name)
            .unwrap()
    }

    pub fn get_source_sha(&'a self, dest_sha: &Oid) -> Oid {
        self.mapper
            .get_translated(Some(*dest_sha), self.dest.name, self.source.name)
            .unwrap()
    }

    pub fn copy_ref_unchecked(
        &'a self,
        ref_name: &str,
        old_source_sha: Option<Oid>,
        new_source_sha: Option<Oid>,
        git_push_opts: Option<Vec<String>>,
    ) -> Option<Oid> {
        debug!("Copying ref {:?} {:?}", old_source_sha, new_source_sha);
        if new_source_sha == old_source_sha {
            return new_source_sha;
        }

        if new_source_sha == None {
            git::delete_remote_branch(self.dest.working, &ref_name,git_push_opts).expect("Could not remove remote reference!");
            return None;
        }

        let commits = self.source
            .get_commits_between(old_source_sha, &new_source_sha.unwrap()); //self.get_commits_to_import(old_upstream_sha, new_upstream_sha);

        commits.into_iter()
            .filter(|&oid| !self.mapper.has_sha(&oid, "upstream", "local"))
            //.take(20)
            .map(|oid| self.copy_commit(&oid))
            .last();

        debug!("Copied commits - now copying branch");
        let new_sha = self.get_dest_sha(&new_source_sha.unwrap());

        let res = git::push_sha_ext(&self.dest.working, ref_name, git_push_opts);

        match &res {
            Ok(_) => (),
            Err(err) => eprint!("{}", &err)
        };

        res.unwrap();

        Some(new_sha)
    }

    pub fn copy_commit(&'a self, source_sha: &Oid) -> Oid {
        debug!("About to export commit {} from {}", source_sha, self.source.bare.path().to_string_lossy());
        // Get the source parents
        let source_commit = self.source
            .bare
            .find_commit(*source_sha)
            .expect(&format!("Couldn't find commit specified! {}", source_sha));
        debug!("Source commit found");
        let source_parent_shas: Vec<Oid> = source_commit.parent_ids().collect();
        source_parent_shas.iter().for_each(|v| debug!("Source commit parent: {}", v));

        // Get the dest parents
        let mut dest_parent_commit_shas = source_parent_shas
            .iter()
            .map(|parent_sha| self.get_dest_sha(parent_sha))
            .collect();
        dedup_vec(&mut dest_parent_commit_shas);
        // use the empty commit as a parent if there would be not parents otherwise
        if dest_parent_commit_shas.is_empty() {
            debug!("Adding empty commit as dest commit parent");
            dest_parent_commit_shas.push(self.empty_sha());
        }

        // Turn merges into fast-forwards where possible
        if dest_parent_commit_shas.len() == 2 {
            let first = dest_parent_commit_shas[0];
            let second = dest_parent_commit_shas[1];
            if self.dest.working.graph_descendant_of(first, second).unwrap() {
                dest_parent_commit_shas = vec![first];
            } else if self.dest.working.graph_descendant_of(second, first).unwrap() {
                dest_parent_commit_shas = vec![second]
            }
        }

        dest_parent_commit_shas.iter().for_each(|v| debug!("Dest commit parent: {}", v));

        // Checkout the first dest parent
        let new_dest_head = *dest_parent_commit_shas.get(0).unwrap();
        self.dest.working.set_head_detached(new_dest_head).unwrap();
        debug!("Checked out the first parent");
        self.dest
            .working
            .checkout_head(Some(CheckoutBuilder::new().force()))
            .unwrap();
        info!("Set head to {}", new_dest_head);

        let n : Vec<&str> = vec!();
        ::util::command_raw(self.source.workdir(), "ls", n.iter()).ok().map(|v|
            trace!("Source files: {}", String::from_utf8(v.stdout).unwrap())
        );
        ::util::command_raw(self.source.workdir(), "ls", ["sub"].iter()).ok().map(|v|
            trace!("Source files: {}", String::from_utf8(v.stdout).unwrap())
        );
        ::util::command_raw(self.dest.workdir(), "ls", n.iter()).ok().map(|v|
            trace!("Dest files: {}", String::from_utf8(v.stdout).unwrap())
        );
        ::util::command_raw(self.dest.workdir(), "ls", ["sub"].iter()).ok().map(|v|
            trace!("Dest files: {}", String::from_utf8(v.stdout).unwrap())
        );

        info!("\nCopying {}, {:?}", source_sha, source_parent_shas);
        let source_parent_tree = if source_parent_shas.len() > 1 {
            dest_parent_commit_shas
                .first()
                .map(|dest_parent_sha| self.get_source_sha(dest_parent_sha))
        } else {
            source_parent_shas
                .first()
                .map(|v| *v)
        }.map(|sha| self.source.bare.find_commit(sha).unwrap().tree().unwrap());

        let diff = self.source
            .bare
            .diff_tree_to_tree(
                source_parent_tree.as_ref(),
                Some(&source_commit.tree().unwrap()),
                None,
            )
            .unwrap();

        let mut changes = false;

        diff.deltas().for_each(|delta| {
            let file_path = delta.new_file().path().expect("Bad git path");
            self.translate(file_path)
                .map(|applicable_path| debug!("{:?} {:?}", delta.status(), applicable_path));
            match delta.status() {
                Delta::Added => {
                    self.translate(file_path).map(|applicable_path| {
                        let parent = applicable_path.parent().expect("This path has a parent");
                        fs::create_dir_all(parent)
                            .expect(&format!("Could not create parent for {:?}", parent));
                        let mut file = OpenOptions::new()
                            .read(true)
                            .write(true)
                            .create(true)
                            .open(&applicable_path)
                            .expect(&format!("Could not open {:?}", applicable_path));
                        let new_blob = self.source
                            .bare
                            .find_object(delta.new_file().id(), Some(ObjectType::Blob))
                            .unwrap()
                            .peel_to_blob()
                            .unwrap();
                        file.write_all(new_blob.content()).unwrap();
                        changes = true;
                    });
                }
                Delta::Modified => {
                    self.translate(file_path).map(|applicable_path| {
                        let mut file = OpenOptions::new()
                            .write(true)
                            .truncate(true)
                            .create(false)
                            .open(&applicable_path)
                            .expect(&format!("Could not open {:?}", applicable_path));
                        let new_blob = self.source
                            .bare
                            .find_object(delta.new_file().id(), Some(ObjectType::Blob))
                            .unwrap()
                            .peel_to_blob()
                            .unwrap();
                        file.write_all(new_blob.content()).unwrap();
                        changes = true
                    });
                }
                Delta::Deleted => {
                    self.translate(file_path).map(|applicable_path| {
                        debug!("Removing {:?}", applicable_path);
                        fs::remove_file(applicable_path).unwrap();
                        changes = true
                    });
                }
                other => {
                    panic!("Cannot handle 'Delta::{:?}'", other);
                }
            }
        });

        if source_parent_shas.len() > 1 {
            debug!(
                "Deduped parent list from {} to {}",
                source_parent_shas.len(),
                dest_parent_commit_shas.len()
            )
        }

        let mut new_dest_sha = new_dest_head;

        // Create a new commit if there are changes to record or if its a merge commit in the destination
        if dest_parent_commit_shas.len() > 1 || changes {
            let mut index = self.dest.working_index();
            index
                .add_all(
                    vec!["."],
                    IndexAddOption::DEFAULT,
                    Some(&mut |path, _| {
                        debug!("Adding {:?} to index", path);
                        0
                    }),
                )
                .unwrap();
            index.write().unwrap();
            let index_tree_oid = index.write_tree().unwrap();
            let index_tree = self.dest.working.find_tree(index_tree_oid).unwrap();
            let parent_commits: Vec<Commit> = dest_parent_commit_shas
                .iter()
                .map(|parent_sha| self.dest.working.find_commit(*parent_sha).unwrap())
                .collect();
            let parent_commits_refs: Vec<&Commit> = parent_commits.iter().collect();

            new_dest_sha = self.dest
                .working
                .commit(
                    Some("HEAD"),
                    &source_commit.author(),
                    &source_commit.committer(),
                    source_commit.message().unwrap(),
                    &index_tree,
                    &parent_commits_refs,
                )
                .unwrap();
        }

        return self.record_sha_update(source_sha, new_dest_sha);
    }
}
