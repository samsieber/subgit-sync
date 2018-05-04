use git2::{Repository, Oid, Delta, IndexAddOption, ResetType, Index, Commit, ObjectType};
use super::map::CommitMapper;
use std::path::{Path, PathBuf};
use std::fs::OpenOptions;
use std::fs::File;
use std::io::Write;
use std::fs;

pub struct GitLocation<'a> {
    pub location: &'a Path,
    pub name: &'static str,
    pub bare: &'a Repository,
    pub working: &'a Repository,
}

pub struct Copier<'a> {
    pub source: GitLocation<'a>,
    pub dest: GitLocation<'a>,
    pub mapper: &'a CommitMapper<'a>
}

impl<'a> GitLocation<'a> {
    fn working_index(&self) -> Index {
        self.working.index().unwrap()
    }

    fn workdir(&self) -> &Path{
        &self.working.workdir().expect("The map repo must have a workdir")
    }

    fn work_path<S: AsRef<str>>(&self, s:S) -> PathBuf {
        return self.workdir().join(s.as_ref());
    }
}

// Removes duplicates from the end of the vec, preserving the first seen order
// Unoptimized, but probably good enough for small vecs
fn dedup_vec<Item: Eq>(items: &mut Vec<Item>){
    if items.len() < 2 { return; }
    let to_drop : Vec<_> = items.iter().enumerate().filter_map(|(idx, item)| {
        if items.iter().skip(idx+1).any(|other_item| other_item == item) {
            Some(idx)
        } else {
            None
        }
    }).collect();
    to_drop.iter().rev().for_each(|idx| {items.remove(*idx);});
}

impl<'a> Copier<'a> {
    fn translate(&'a self, path: &Path) -> Option<PathBuf>{
        let chopped = path.strip_prefix(self.source.location).ok();
        chopped.map(|new_path| self.dest.workdir().join(self.dest.location.join(new_path)))
    }

    fn record_sha_update(&'a self, source_sha: &Oid, dest_sha: Oid) -> Oid {
        info!("Mapping {:?} <-> {:?}", source_sha, dest_sha);
        self.mapper.set_translated(source_sha, self.source.name, self.dest.name, &dest_sha);
        self.mapper.set_translated(&dest_sha, self.dest.name, self.source.name, source_sha);
        dest_sha
    }

    fn empty_sha(&'a self) -> Oid{
        self.dest.working
            .find_reference("refs/sync/empty").expect("An empty commit should exist")
            .target().expect("An empty commit should be an oid, not another reference")
    }


    pub fn get_dest_sha(&'a self, source_sha: &Oid) -> Oid {
        self.mapper.get_translated(Some(*source_sha), self.source.name, self.dest.name).unwrap()
    }

    fn get_dest_commit(&'a self, source_sha: &Oid) -> Commit {
        self.dest.working.find_commit(self.mapper.get_translated(Some(*source_sha), self.source.name, self.dest.name).unwrap()).unwrap()
    }

    pub fn copy_commit(&'a self, source_sha: &Oid) -> Oid {
        // Get the source parents
        let source_commit = self.source.bare.find_commit(*source_sha).expect("Couldn't find commit specified!");
        let source_parent_shas : Vec<Oid> = source_commit.parent_ids().collect();

        // Get the dest parents  - use the empty commit as a parent if there would be not parents otherwise
        let mut dest_parent_commit_shas = source_parent_shas.iter().map(|parent_sha| self.get_dest_sha(parent_sha)).collect();
        dedup_vec(&mut dest_parent_commit_shas);
        if dest_parent_commit_shas.is_empty() {
            dest_parent_commit_shas.push(self.empty_sha());
        }
        let dest_parent_commits : Vec<Commit> = dest_parent_commit_shas.iter().map(|parent_sha| self.dest.working.find_commit(*parent_sha).unwrap()).collect();

        // Checkout the first dest parent
        let new_dest_head = *dest_parent_commit_shas.get(0).unwrap();
        self.dest.working.set_head_detached(new_dest_head);
        self.dest.working.reset(dest_parent_commits.first().unwrap().as_object(), ResetType::Hard, None).unwrap();
        info!("Set head to {}", new_dest_head);

        info!("\nImporting {}, {:?}", source_sha, source_parent_shas);
        let source_parent_tree = source_parent_shas.first().map(|sha| self.source.bare.find_commit(*sha).unwrap().tree().unwrap());
        let diff = self.source.bare.diff_tree_to_tree(source_parent_tree.as_ref(), Some(&source_commit.tree().unwrap()), None).unwrap();

        let mut changes = false;

        diff.deltas().for_each(|delta| {
            let file_path = delta.new_file().path().expect("Bad git path");
            self.translate(file_path).map(|applicable_path|
                                              debug!("{:?} {:?}", delta.status(), applicable_path)
            );
            match delta.status() {
                Delta::Added => {
                    self.translate(file_path).map(|applicable_path|{
                        let parent = applicable_path.parent().expect("This path has a parent");
                        fs::create_dir_all(parent).expect(&format!("Could not create parent for {:?}", parent));
                        let mut file = OpenOptions::new()
                            .read(true)
                            .write(true)
                            .create(true)
                            .open(&applicable_path)
                            .expect(&format!("Could not open {:?}", applicable_path));
                        let new_blob = self.source.bare.find_object(delta.new_file().id(), Some(ObjectType::Blob)).unwrap().peel_to_blob().unwrap();
                        file.write_all(new_blob.content());
                        changes = true;
                    });
                }
                Delta::Modified => {
                    self.translate(file_path).map(|applicable_path|{
                        let mut file = OpenOptions::new()
                            .write(true)
                            .truncate(true)
                            .create(false)
                            .open(&applicable_path)
                            .expect(&format!("Could not open {:?}", applicable_path));
                        let new_blob = self.source.bare.find_object(delta.new_file().id(), Some(ObjectType::Blob)).unwrap().peel_to_blob().unwrap();
                        file.write_all(new_blob.content());
                        changes = true
                    });
                }
                Delta::Deleted => {
                    self.translate(file_path).map(|applicable_path|{
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

        if (source_parent_shas.len() > 1) {
            debug!("Deduped parent list from {} to {}", source_parent_shas.len(), dest_parent_commit_shas.len())
        }

        let mut new_dest_sha = new_dest_head;

        // Create a new commit if there are changes to record or if its a merge commit in the destination
        if source_parent_shas.len() > 1 || changes {
            let mut index = self.dest.working_index();
            index.add_all(vec!("."), IndexAddOption::DEFAULT, Some(& mut|path, _| {debug!("Adding {:?} to index", path); 0})).unwrap();
            index.write().unwrap();
            let index_tree_oid = index.write_tree().unwrap();
            let index_tree = self.dest.working.find_tree(index_tree_oid).unwrap();
            let signature = self.dest.working.signature().unwrap();
            let parent_commits : Vec<Commit> = dest_parent_commit_shas.iter().map(|parent_sha| self.dest.working.find_commit(*parent_sha).unwrap()).collect();
            let parent_commits_refs : Vec<&Commit> = parent_commits.iter().collect();

            new_dest_sha = self.dest.working.commit(Some("HEAD"), &source_commit.author(), &source_commit.committer(), source_commit.message().unwrap(), &index_tree, &parent_commits_refs).unwrap();
        }

        return self.record_sha_update(source_sha, new_dest_sha);
    }
}