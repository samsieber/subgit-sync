use git2::{Oid, Repository};
use std::str;
use Path;
use hex;
use fs;

pub struct CommitMapper<'a> {
    pub map: &'a Repository,
}

fn sha_path(sha: &Oid) -> String {
    let depth = 4;
    let sha_length = 40;
    let mut sha_path = String::with_capacity(sha_length + depth);

    hex::encode(sha.as_bytes())
        .as_str()
        .bytes()
        .enumerate()
        .for_each(|(idx, u)| {
            sha_path.push(u as char);
            if idx < depth {
                sha_path.push('/');
            }
        });

    sha_path
}

impl<'a> CommitMapper<'a> {
    fn workdir(&self) -> &Path {
        &self.map
            .workdir()
            .expect("The map repo must have a workdir")
    }

    pub fn get_translated<SN: AsRef<str>, DN: AsRef<str>>(
        &'a self,
        maybe_sha: Option<Oid>,
        source: SN,
        dest: DN,
    ) -> Option<Oid> {
        let sha = match maybe_sha {
            Some(ref sha_) => sha_,
            None => return None,
        };
        let partial_path = sha_path(sha);
        let full_path = format!("{}_to_{}/{}", source.as_ref(), dest.as_ref(), partial_path);
        let content = fs::content_of_file_if_exists(&fs::make_absolute(&self.workdir())
            .unwrap()
            .join(full_path));
        content.map(|oid_str| {
            Oid::from_bytes(&hex::decode(oid_str).unwrap())
                .expect("The format should be correct for a stored sha")
        })
    }

    pub fn has_sha<SN: AsRef<str>, DN: AsRef<str>>(
        &'a self,
        sha: &Oid,
        source: SN,
        dest: DN,
    ) -> bool {
        let partial_path = sha_path(sha);
        let full_path = format!("{}_to_{}/{}", source.as_ref(), dest.as_ref(), partial_path);
        fs::make_absolute(&self.workdir())
            .unwrap()
            .join(full_path)
            .exists()
    }

    pub fn set_translated<SN: AsRef<str>, DN: AsRef<str>>(
        &'a self,
        sha: &Oid,
        source: SN,
        dest: DN,
        translated: &Oid,
    ) {
        let partial_path = sha_path(sha);
        let full_path = format!("{}_to_{}/{}", source.as_ref(), dest.as_ref(), partial_path);
        fs::write_content_to_file(
            &fs::make_absolute(&self.workdir()).unwrap().join(full_path),
            &format!("{}", translated),
        );
    }
}
