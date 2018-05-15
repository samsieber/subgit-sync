use std::path::{PathBuf, Path};
use std::fs::{remove_dir_all, create_dir_all};
use util;
use std::error::Error;
use fs;
use std;

pub type Res<T> = Result<T, Box<Error>>;

pub fn clear_path<P: AsRef<Path>>(path: P) {
    if path.as_ref().exists() {
        remove_dir_all(&path).unwrap();
    }
    create_dir_all(&path).unwrap();
}

pub fn test_dir(path: &'static str) -> PathBuf {
    let p = PathBuf::from(format!("test_data/{}", path));
    clear_path(&p);
    p
}

pub fn init_bare_repo<P: AsRef<Path>>(name: &str, parent: P) -> Result<PathBuf, Box<Error>> {
    util::command(&parent, "git", ["init", "--bare", name].iter())?;
    Ok(PathBuf::from(format!("{}/{}", parent.as_ref().to_string_lossy(), name)))
}

pub fn clone<P: AsRef<Path>, CWD: AsRef<Path>>(cwd: CWD, p: P) -> Res<PathBuf>{
    let fps = p.as_ref().to_string_lossy();
    let fp = fps.split("/").last().unwrap();
    let name = fp.split(".").nth(0).unwrap();
    let full_path = format!("file://{}", fs::make_absolute(p.as_ref())?.to_string_lossy());
    util::command(&cwd, "git", &["clone", &full_path])?;
    Ok(cwd.as_ref().join(name).to_owned())
}

pub fn assert_works<F: FnOnce() -> Result<(), Box<Error>>>(f:F) {
    f().unwrap();
}

pub fn assert_contents_equal<P1: AsRef<Path>, P2: AsRef<Path>, R1: AsRef<Path>, R2: AsRef<Path>>(root1: R1, sub_path1: P1, root2: R2, sub_path2: P2){
    let s1 = std::fs::read_to_string(root1.as_ref().join(&sub_path1)).unwrap();
    let s2 = std::fs::read_to_string(root2.as_ref().join(&sub_path2)).unwrap();
    assert_eq!(s1, s2);
}

pub fn assert_dir_content_equal<D1: AsRef<Path>, D2: AsRef<Path>>(origin: D1, comp: D2) {
    let raw = util::command_raw(
        std::env::current_dir().unwrap(),
        "diff",
        ["--exclude=.git", &origin.as_ref().to_string_lossy(), &comp.as_ref().to_string_lossy()].iter()
    ).unwrap();
    assert_eq!("", &String::from_utf8(raw.stdout).unwrap());
}

pub fn set_credentials<P: AsRef<Path>>(path: P){
    util::command(&path.as_ref(), "git", ["config", "user.name", "test user"].iter()).unwrap();
    util::command(&path.as_ref(), "git", ["config", "user.email", "test@example.com"].iter()).unwrap();
}
