use git2::{Commit, ObjectType, Oid, Repository, Signature};
use git2;
use Error;
use std::path::Path;

pub fn open_or_clone_bare<P: AsRef<Path>>(path: P, url: &str) -> Repository {
    match Repository::open_bare(&path) {
        Ok(repo) => repo,
        Err(e) => {
            info!(
                "Couldn't open repo at {}, attempting clone from {}. Original error: {:?}",
                &path.as_ref().to_string_lossy(),
                &url,
                e
            );
            let mut builder = git2::build::RepoBuilder::new();
            match builder.bare(true).clone(url, path.as_ref()) {
                Ok(repo) => repo,
                Err(e) => panic!("failed to open or clone clone: {}", e),
            }
        }
    }
}

pub fn commit_empty(
    repo: &Repository,
    author: &Signature,
    committer: &Signature,
    message: &str,
    parents: &[&Commit],
) -> Result<Oid, Box<Error>> {
    let new_empty_index_oid = repo.index()?.write_tree()?;
    let new_empty_object = repo.find_object(new_empty_index_oid, Some(ObjectType::Tree))?;
    let new_empty_tree = new_empty_object.as_tree().unwrap();

    Ok(repo.commit(
        Some("HEAD"),
        &author,
        &committer,
        &message,
        new_empty_tree,
        &parents,
    )?)
}

pub fn get_refs(repo: &Repository, glob: &str) -> Result<Vec<(String, Oid)>, Box<Error>> {
    let ref_list: Result<Vec<(String, Oid)>, _> = repo.references_glob(glob)?
        .map(|r| {
            let rr = r?;
            Ok((
                rr.name().expect("can't get ref name").to_owned(),
                rr.target().expect("Can't get target"),
            ))
        })
        .collect();
    ref_list
}

pub fn is_standard<S: AsRef<str>>(value: &S) -> bool {
    value.as_ref().starts_with("refs/heads") || value.as_ref() == "HEAD"
}
