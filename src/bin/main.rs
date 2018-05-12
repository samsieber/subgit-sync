extern crate git2;
#[macro_use]
extern crate log;
extern crate subgit_rs;

fn main() {
    subgit_rs::setup_logging();
    let res = subgit_rs::run_import_test(
        "test1",
        "https://github.com/git-repo-samples/javalgorithms.git",
        "Others",
    );

    //    let res = subgit_rs::run_import_test(
    //        "test2",
    //        "https://github.com/git-repo-samples/kotlin-dsl.git",
    //        "provider",
    //    );
    info!("{:?}", res);
    res.expect("Failed test");
}
