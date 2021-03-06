#![allow(unused)]

use std;
use std::error::Error;
use std::ffi::OsStr;
use std::fs::{create_dir_all, remove_dir_all};
use std::path::{Path, PathBuf};
use std::process::Output;
use subgit_sync::{make_absolute, StringError};
use std::env;
use std::time::Duration;

pub fn write_files<P, K, V, I>(root: P, files: I) -> Result<(), Box<Error>>
    where
        P: AsRef<Path>,
        K: AsRef<Path>,
        V: AsRef<[u8]>,
        I: IntoIterator<Item = (K, V)>,
{
    let root_dir = root.as_ref();

    for (f, c) in files {
        std::fs::create_dir_all(root_dir.join(&f).parent().unwrap())?;
        std::fs::write(root_dir.join(&f), c)?;
    }

    Ok(())
}

pub type Res<T> = Result<T, Box<Error>>;

pub fn clear_path<P: AsRef<Path>>(path: P) {
    if path.as_ref().exists() {
        remove_dir_all(&path).unwrap();
    }
    create_dir_all(&path).unwrap();
}


pub fn init_bare_repo<P: AsRef<Path>>(name: &str, parent: P) -> Result<PathBuf, Box<Error>> {
    command(&parent, "git", ["init", "--bare", name].iter())?;
    Ok(PathBuf::from(format!(
        "{}/{}",
        parent.as_ref().to_string_lossy(),
        name
    )))
}

pub fn clone<P: AsRef<Path>, CWD: AsRef<Path>>(cwd: CWD, p: P) -> Res<PathBuf> {
    let fps = p.as_ref().to_string_lossy();
    let fp = fps.split("/").last().unwrap();
    let name = fp.split(".").nth(0).unwrap();
    let full_path = format!("file://{}", make_absolute(p.as_ref())?.to_string_lossy());
    command(&cwd, "git", &["clone", &full_path])?;
    Ok(cwd.as_ref().join(name).to_owned())
}

pub fn clone_url<CWD: AsRef<Path>, URL: AsRef<str>, STR: AsRef<str>>(cwd: CWD, url: URL, name: STR) -> Res<PathBuf> {
    command(&cwd, "git", &["clone", url.as_ref(), &name.as_ref()])?;
    Ok(cwd.as_ref().join(name.as_ref()).to_owned())
}

pub fn assert_works<F: FnOnce() -> Result<(), Box<Error>>>(f: F) {
    f().unwrap();
}

pub fn assert_file_equals<P1: AsRef<Path>, P2: AsRef<Path>, C: AsRef<str>>(root: P1, sub_path: P2, content: C) {
    let s1 = std::fs::read_to_string(root.as_ref().join(&sub_path)).unwrap();
    assert_eq!(&s1, content.as_ref());
}

pub fn assert_contents_equal<P1: AsRef<Path>, P2: AsRef<Path>, R1: AsRef<Path>, R2: AsRef<Path>>(
    root1: R1,
    sub_path1: P1,
    root2: R2,
    sub_path2: P2,
) {
    let s1 = std::fs::read_to_string(root1.as_ref().join(&sub_path1)).unwrap();
    let s2 = std::fs::read_to_string(root2.as_ref().join(&sub_path2)).unwrap();
    assert_eq!(s1, s2);
}

pub fn compare_dir_content<D1: AsRef<Path>, D2: AsRef<Path>>(origin: D1, comp: D2) -> String {
    let raw = command_raw(
        std::env::current_dir().unwrap(),
        "diff",
        [
            "-r",
            "--exclude=.git",
            &origin.as_ref().to_string_lossy(),
            &comp.as_ref().to_string_lossy(),
        ]
            .iter(),
    )
        .unwrap();
    String::from_utf8(raw.stdout).unwrap()
}

pub fn assert_dir_content_equal<D1: AsRef<Path>, D2: AsRef<Path>>(origin: D1, comp: D2) {
    let raw = command_raw(
        std::env::current_dir().unwrap(),
        "diff",
        [
            "-r",
            "--exclude=.git",
            &origin.as_ref().to_string_lossy(),
            &comp.as_ref().to_string_lossy(),
        ]
            .iter(),
    )
        .unwrap();
    assert_eq!("", &String::from_utf8(raw.stdout).unwrap());
}

pub fn set_credentials<P: AsRef<Path>>(path: P, name: &str) {
    command(
        &path.as_ref(),
        "git",
        ["config", "user.name", &format!("User {}", name)].iter(),
    )
        .unwrap();
    command(
        &path.as_ref(),
        "git",
        ["config", "user.email", &format!("{}@example.com", name)].iter(),
    )
        .unwrap();
}

pub fn command<P, C, I, S>(path: P, command: C, args: I) -> Result<(), Box<Error>>
    where
        P: AsRef<Path>,
        C: AsRef<OsStr>,
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
{
    let result = command_raw(path, command, args)?;

    if !result.status.success() {
        let err_message = format!(
            "Could not execute command {}. Full command output: \nStd Out:\n{}\nStd Err:\n{}",
            &result.status,
            String::from_utf8(result.stdout)?,
            String::from_utf8(result.stderr)?
        );

        println!("{}", err_message);

        return Err(Box::new(StringError {
            message: err_message,
        }));
    } else {
        println!("{}", String::from_utf8(result.stdout)?);
        println!("{}", String::from_utf8(result.stderr)?);
    }

    Ok(())
}

pub fn command_raw<P, C, I, S>(path: P, command: C, args: I) -> Result<Output, Box<Error>>
    where
        P: AsRef<Path>,
        C: AsRef<OsStr>,
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
{
    let mut process = std::process::Command::new(&command);
    process
        .env_clear()
        .env("PATH", std::env::var("PATH").unwrap());

    process.args(args);
    process.current_dir(path.as_ref());

    Ok(process.output()?)
}

/// Get top level working directory
pub fn get_top() -> PathBuf {
    get_top_dir(&get_target_dir()).to_owned()
}

// Get absolute path to the "target" directory ("build" dir)
fn get_target_dir() -> PathBuf {
    let bin = env::current_exe().expect("exe path");
    let mut target_dir = PathBuf::from(bin.parent().expect("bin parent"));
    while target_dir.file_name() != Some(OsStr::new("target")) {
        target_dir.pop();
    }
    target_dir
}
// Get absolute path to the project's top dir, given target dir
fn get_top_dir<'a>(target_dir: &'a Path) -> &'a Path {
    target_dir.parent().expect("target parent")
}

pub fn sleep(seconds: u32) {
    std::thread::sleep(Duration::new(seconds as u64, 0));
}