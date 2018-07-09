use git2::{Commit, Delta, Index, IndexAddOption, ObjectType, Oid, Repository, build::CheckoutBuilder, ResetType};
use super::map::CommitMapper;
use std::path::{Path, PathBuf};
use std::fs::OpenOptions;
use std::io::Write;
use std::fs;
use git;
use action::PushListener;

pub struct GitLocation<'a> {
    pub location: &'a Path,
    pub name: &'static str,
    pub bare: &'a Repository,
    pub working: &'a Repository,
}

pub struct Copier<'a> {
    pub source: GitLocation<'a>,
    pub dest: GitLocation<'a>,
    pub mapper: CommitMapper<'a>,
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
        starting_shas: Vec<Oid>,
        dest_sha_inclusive: &Oid,
    ) -> Vec<Oid> {
        let walker = &mut self.bare.revwalk().unwrap();
        starting_shas.iter().for_each(|v| info!("Using {} as a base commit to exclude", v));
        starting_shas.into_iter().for_each(|starting_sha| walker.hide(starting_sha).unwrap());
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

    fn get_unseen_source_commits_between(
        &self,
        maybe_starting_sha: Option<Oid>,
        dest_sha_inclusive: &Oid,
    ) -> Vec<Oid> {
        debug!("Finding commits in {} between {:?} and {:?}", self.source.bare.path().to_string_lossy(), maybe_starting_sha, dest_sha_inclusive);
        if let Some(starting_sha) = maybe_starting_sha {
            self.source
                .get_commits_between(vec!(starting_sha), dest_sha_inclusive)
        } else {
            let mut starting_shas : Vec<Oid> = git::get_n_recent_shas(self.dest.bare, 10);
            starting_shas.push(self.dest.working.find_reference("HEAD").unwrap().peel_to_commit().unwrap().id());
            starting_shas = starting_shas.iter().map(|sha| self.get_source_sha(sha)).collect();
            self.source
                .get_commits_between(starting_shas, dest_sha_inclusive)
        }
    }

    fn translate(&'a self, path: &Path) -> Option<PathBuf> {
        let chopped = path.strip_prefix(self.source.location).ok();
        chopped.map(|new_path| self.dest.workdir().join(self.dest.location.join(new_path)))
    }

    fn record_sha_update(&'a self, source_sha: &Oid, dest_sha: Oid) -> Oid {
        info!("Mapping {} <-> {} ({} <-> {})", source_sha, dest_sha, self.source.name, self.dest.name);
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

    fn get_dest_sha(&'a self, source_sha: &Oid) -> Oid {
        self.mapper
            .get_translated(Some(*source_sha), self.source.name, self.dest.name)
            .unwrap()
    }

    fn get_source_sha(&'a self, dest_sha: &Oid) -> Oid {
        self.mapper
            .get_translated(Some(*dest_sha), self.dest.name, self.source.name)
            .unwrap()
    }

    pub fn import_initial_empty_commits(&'a self){
        let commits_to_import = git::find_safe_empty_na_commits(self.source.bare, self.source.location.to_string_lossy().as_ref());
        if let Some(first_oid) = commits_to_import.first() {
            info!("Importing {} empty commits", commits_to_import.len());
            let empty_dest_sha = self.empty_sha();
            commits_to_import.iter().for_each(|empty_source_sha| {
                self.mapper.set_translated(empty_source_sha, self.source.name, self.dest.name, &empty_dest_sha);
            });
            self.mapper.set_translated(&empty_dest_sha, self.dest.name, self.source.name, first_oid);
        }
    }

    pub fn copy_ref_unchecked<PL: PushListener>(
        &'a self,
        ref_name: &str,
        supposed_old_source_sha: Option<Oid>,
        new_source_sha: Option<Oid>,
        force_push: bool,
        git_push_opts: Option<Vec<String>>,
        push_listener: Option<PL>,
    ) -> Option<Oid> {
        debug!("Copying ref {:?} {:?}", supposed_old_source_sha, new_source_sha);
        if new_source_sha == None {
            if let Some(pl) = &push_listener {
                pl.pre_push(&ref_name, git::no_sha());
            }
            git::delete_remote_branch(self.dest.working, &ref_name,git_push_opts).expect("Could not remove remote reference!");
            if let Some(pl) = &push_listener {
                pl.post_push(&ref_name, git::no_sha());
            }
            return None;
        }

        let old_source_sha = supposed_old_source_sha.and_then(|source_sha| {
            if self.mapper.get_translated(Some(source_sha), self.dest.name, self.source.name ).is_some() {
                Some(source_sha)
            } else {
                self.dest.bare.find_reference(ref_name).ok().and_then(|reference| {
                    let old_dest_sha = reference.peel_to_commit().unwrap().id();
                    self.mapper.get_translated(Some(old_dest_sha), self.dest.name, self.source.name)
                })
            }
        });

        if new_source_sha == old_source_sha {
            return new_source_sha;
        }

        let commits = self.get_unseen_source_commits_between(old_source_sha, &new_source_sha.unwrap()); //self.get_commits_to_import(old_upstream_sha, new_upstream_sha);

        let total_commits = commits.len();
        let mut current_commit = 0;

        debug!("Need to import {} commits for {}", &total_commits, &ref_name);

        commits.into_iter()
            .filter(|&oid| {
                current_commit += 1;
                if !self.mapper.has_sha(&oid, self.source.name, self.dest.name){
                    debug!("Copying Commit ({}/{})", &current_commit, &total_commits);
                    true
                } else {
                    debug!("Skipping Commit ({}/{}) - already imported", &current_commit, &total_commits);
                    false
                }
            })
            //.take(20)
            .map(|oid| self.copy_commit(&oid))
            .last();

        debug!("Copied commits - now copying branch");
        let new_sha = self.get_dest_sha(&new_source_sha.unwrap());
        self.dest.working.reset(&self.dest.working.find_object(new_sha, None).unwrap(), ResetType::Hard, None).unwrap();

        debug!("Source was {}, now assigning to {} in dest", &new_source_sha.unwrap(), &new_sha);

        if let Some(pl) = &push_listener {
            pl.pre_push(&ref_name, new_sha);
        }
        let res = git::push_sha_ext(&self.dest.working, ref_name, force_push, git_push_opts);
        if let Some(pl) = &push_listener {
            pl.post_push(&ref_name, new_sha);
        }

        match &res {
            Ok(_) => (),
            Err(err) => eprint!("{}", &err)
        };

        res.unwrap();

        Some(new_sha)
    }

    pub fn copy_commit(&'a self, source_sha: &Oid) -> Oid {
        debug!("Copying commit {} from '{}' to '{}'", source_sha, self.source.name, self.dest.name);
        // Get the source parents
        let source_commit = self.source
            .bare
            .find_commit(*source_sha)
            .expect(&format!("Couldn't find commit specified! {}", source_sha));
        trace!("Source commit found");
        let source_parent_shas: Vec<Oid> = source_commit.parent_ids().collect();
        debug!("Source parents: {:?}", source_parent_shas);

        // Get the dest parents
        let mut dest_parent_commit_shas = source_parent_shas
            .iter()
            .map(|parent_sha| self.get_dest_sha(parent_sha))
            .collect();
        dedup_vec(&mut dest_parent_commit_shas);
        // use the empty commit as a parent if there would be not parents otherwise
        if dest_parent_commit_shas.is_empty() {
            debug!("Adding empty commit as dest commit parent: {}", self.empty_sha());
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

        debug!("Dest parents: {:?}", dest_parent_commit_shas);

        // Checkout the first dest parent
        let new_dest_head = *dest_parent_commit_shas.get(0).unwrap();
        self.dest.working.set_head_detached(new_dest_head).unwrap();
        debug!("Checked out the first parent in dest: {}", new_dest_head);
        self.dest
            .working
            .checkout_head(Some(CheckoutBuilder::new().force()))
            .unwrap();
        info!("Set head to {}", new_dest_head);

        info!("Copying {} with source parents of {:?}", source_sha, source_parent_shas);
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
