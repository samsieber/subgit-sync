use std;
use std::error::Error;
use std::fs::symlink_metadata;
use std::fs::File;
use std::fs::OpenOptions;
use std::io::Read;
use std::io::Write;
use std::path::{Path, PathBuf};

pub use std::fs::{copy, create_dir, create_dir_all, remove_dir_all};

#[allow(unused)]
pub fn remove_if_exists<P: AsRef<Path>>(path: P) -> Result<(), Box<Error>> {
    if symlink_metadata(path.as_ref()).is_ok() {
        remove_dir_all(&path)?;
    }
    Ok(())
}

pub fn content_of_file_if_exists<P: AsRef<Path>>(path: &P) -> Option<String> {
    let path: &Path = path.as_ref();
    if !path.exists() {
        return None;
    }

    let mut contents = String::new();
    let mut file = File::open(path).expect("Unable to open file");
    file.read_to_string(&mut contents)
        .expect("Unable to read data");

    Some(contents)
}

pub fn write_content_to_file<P: AsRef<Path>, S: AsRef<str>>(path: &P, content: &S) {
    let path: &Path = path.as_ref();

    std::fs::create_dir_all(path.parent().expect("Parent surely exists")).unwrap();

    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(&path)
        .expect(&format!("Could not open {:?}", path));

    file.write_all(content.as_ref().as_bytes())
        .expect("Unable to write data");
}

pub fn make_absolute<P: AsRef<Path>>(relative_path: P) -> Result<PathBuf, Box<Error>> {
    let mut abs_path = std::env::current_dir()?;
    abs_path.push(&relative_path);
    Ok(abs_path)
}

pub fn symlink_dirs<SP: AsRef<Path>, DP: AsRef<Path>>(
    source: &SP,
    dest: &DP,
    dirs: &Vec<&str>,
) -> Result<(), Box<Error>> {
    let abs_source = make_absolute(&source)?;
    let links: Result<Vec<()>, _> = dirs
        .iter()
        .map(|&dir| -> Result<(), Box<Error>> {
            Ok(std::os::unix::fs::symlink(
                abs_source.join(dir),
                dest.as_ref().join(dir),
            )?)
        })
        .collect();
    links?;
    Ok(())
}
