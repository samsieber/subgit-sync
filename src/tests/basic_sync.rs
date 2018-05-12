use std::path::{PathBuf, Path};
use std::fs::{remove_dir_all, create_dir_all};
use util;
use std::error::Error;
use fs;
use model::WrappedSubGit;
use log::LevelFilter;
use model::BinSource;
use simplelog::TermLogger;
use simplelog::Config;
use std;


type Res<T> = Result<T, Box<Error>>;

fn clear_path<P: AsRef<Path>>(path: P) {
    if path.as_ref().exists() {
        remove_dir_all(&path).unwrap();
    }
    create_dir_all(&path).unwrap();
}

fn test_dir(path: &'static str) -> PathBuf {
    let p = PathBuf::from(format!("test_data/{}", path));
    clear_path(&p);
    p
}

fn init_bare_repo(name: &str, parent: &PathBuf) -> Result<PathBuf, Box<Error>> {
    util::command(parent, "git", ["init", "--bare", name].iter())?;
    Ok(PathBuf::from(format!("{}/{}", parent.to_string_lossy(), name)))
}

fn clone<P: AsRef<Path>, CWD: AsRef<Path>>(cwd: CWD, p: P) -> Res<PathBuf>{
    let fps = p.as_ref().to_string_lossy();
    let fp = fps.split("/").last().unwrap();
    let name = fp.split(".").nth(0).unwrap();
    let full_path = format!("file://{}", fs::make_absolute(p.as_ref())?.to_string_lossy());
    util::command(&cwd, "git", &["clone", &full_path])?;
    Ok(cwd.as_ref().join(name).to_owned())
}

fn assert_works<F: FnOnce() -> Result<(), Box<Error>>>(f:F) {
    f().unwrap();
}

fn assert_contents_equal<P1: AsRef<Path>, P2: AsRef<Path>, R1: AsRef<Path>, R2: AsRef<Path>>(root1: R1, sub_path1: P1, root2: R2, sub_path2: P2){
    let s1 = std::fs::read_to_string(root1.as_ref().join(&sub_path1)).unwrap();
    let s2 = std::fs::read_to_string(root2.as_ref().join(&sub_path2)).unwrap();
    assert_eq!(s1, s2);
}

fn assert_dir_content_equal<D1: AsRef<Path>, D2: AsRef<Path>>(origin: D1, comp: D2) {
    let raw = util::command_raw(
        std::env::current_dir().unwrap(),
        "diff",
        ["--exclude=.git", &origin.as_ref().to_string_lossy(), &comp.as_ref().to_string_lossy()].iter()
    ).unwrap();
    assert_eq!("", &String::from_utf8(raw.stdout).unwrap());
}


fn run() -> Res<()> {
    let _ = TermLogger::init(LevelFilter::Debug, Config::default());
//    let empty : Vec<String> = vec!();
    let d = test_dir("test_basic");
    let up_bare = init_bare_repo("test.git", &d)?;
    let up = clone(&d, &up_bare)?;
    {
        util::write_files(&up, hashmap!{ "sub/testing" => "This is another test. Yaya." }.iter())?;
        util::command(&up, "git", ["add", "."].iter())?;
        util::command(&up, "git", ["commit", "-m", "This is the first test"].iter())?;
        util::command(&up, "git", ["push"].iter())?;
    };
    let local_bare = init_bare_repo("local.git", &d)?;
    let wrapped = WrappedSubGit::run_creation(
        &local_bare,
        &up_bare,
        "sub",
        None,
        LevelFilter::Debug,
        BinSource {
            location: PathBuf::from("target/debug/hook"),
            symlink: true,
        },
        None,
        None,
    )?;

    wrapped.update_all_from_upstream()?;

    let local = clone(&d, &local_bare)?;
    assert_dir_content_equal(&local, &up.join("sub"));

    {
        util::write_files(&local, hashmap!{ "testing" => "Overwritten in local" }.iter())?;

        util::command(&local, "git", ["add", "."].iter())?;
        util::command(&local, "git", ["commit", "-m", "This is the second test"].iter())?;
        util::command(&local, "git", ["push"].iter())?;

        util::command(&up, "git", ["pull"].iter())?;
    };

    assert_dir_content_equal(&local, &up.join("sub"));

    {
        util::write_files(&up, hashmap!{ "testing" => "Overwritten in upstream" }.iter())?;

        util::command(&up, "git", ["add", "."].iter())?;
        util::command(&up, "git", ["commit", "-m", "This is the third test"].iter())?;
        util::command(&up, "git", ["push"].iter())?;
    };

    Ok(())
}

#[test]
fn test_push_from_local_then_pull_upstream(){
    assert_works(run);
}