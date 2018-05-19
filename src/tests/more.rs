use super::test_util::*;
use super::advanced::*;

#[test]
pub fn test_single_file() {
    let test = TestWrapper::new("single_one", |upstream| {
        upstream.update_working(vec![
            FileAction::overwrite("sub/hello.txt", "Hello world (from upstream)"),
        ]);
        upstream.add(".").unwrap();
        upstream.commit("First Commit from Upstream").unwrap();
        upstream.push().unwrap();
    }, "sub").unwrap();

    test.verify_push_changes_and_pull_in_other(GitType::Upstream, vec![
        FileAction::overwrite("sub/hello.txt", "Hello world (from upstream - again)"),
    ], "Second Commit from Upstream");

    test.verify_push_changes_and_pull_in_other(GitType::Subgit, vec![
        FileAction::overwrite("hello.txt", "Hello world (from subgit)"),
    ], "First Commit from Subgit");

    test.verify_push_changes_and_pull_in_other(GitType::Upstream, vec![
        FileAction::overwrite("sub/hello.txt", "Hello world (from upstream - yet again)"),
    ], "Third Commit from Upstream");
}

#[test]
pub fn test_modify_remove() {

}