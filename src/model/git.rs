use git2::{Repository, Signature, Commit, Oid, ObjectType};
use Error;

pub fn commit_empty(repo: &Repository, signature: &Signature, message: &str, parents: &[&Commit]) -> Result<Oid, Box<Error>>{
    let new_empty_index_oid = repo.index()?.write_tree()?;
    let new_empty_object = repo.find_object(new_empty_index_oid, Some(ObjectType::Tree))?;
    let new_empty_tree = new_empty_object.as_tree().unwrap();

    Ok(repo.commit(Some("HEAD"), &signature, &signature, &message, new_empty_tree, &parents)?)
}

pub fn get_refs(repo: &Repository, glob: &str) -> Result<Vec<(String, Oid)>, Box<Error>>{
    let ref_list : Result<Vec<(String, Oid)>, _> = repo
        .references_glob("refs/heads/*")?
        .map(|r| {
            let rr = r?;
            Ok((rr.name().expect("can't get ref name").to_owned(), rr.target().expect("Can't get target")))
        })
        .collect();
    ref_list
}

pub fn is_standard<S: AsRef<str>>(value: &S) -> bool{
    value.as_ref().starts_with("refs/heads") || value.as_ref() == "HEAD"
}