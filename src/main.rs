#[macro_use]
extern crate log;
extern crate subgit_rs;
extern crate git2;

fn main() {
    subgit_rs::setup_logging();
    let res = subgit_rs::run_import_test("test1", "https://github.com/git-repo-samples/javalgorithms.git", "Others");
    info!("{:?}", res);
    res.expect("Failed test");
}
