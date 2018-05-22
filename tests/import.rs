extern crate subgit_rs;
extern crate log;
extern crate simplelog;

mod harness;
mod util;

use harness::*;

#[test]
pub fn import_single_a_commit() {
    TestWrapper::new("import_single_a_commit", |upstream| {
        upstream.update_working(vec![
            FileAction::overwrite("sub/hello.txt", "Hello world (from upstream)"),
        ]);
        upstream.add(".").unwrap();
        upstream.commit("First Commit from Upstream").unwrap();
        upstream.push().unwrap();
    }, "sub").unwrap();
}

#[test]
pub fn import_two_a_commits() {
    TestWrapper::new("import_two_a_commits", |upstream| {
        upstream.update_working(vec![
            FileAction::overwrite("sub/hello.txt", "Hello world (from upstream)"),
        ]);
        upstream.add(".").unwrap();
        upstream.commit("First Commit from Upstream").unwrap();

        upstream.update_working(vec![
            FileAction::overwrite("sub/again.txt", "Hello world (from upstream again)"),
        ]);
        upstream.add(".").unwrap();
        upstream.commit("Second Commit from Upstream").unwrap();

        upstream.push().unwrap();
    }, "sub").unwrap();
}

#[test]
pub fn import_a_na_2_commits() {
    TestWrapper::new("import_a_na_2_commits", |upstream| {
        upstream.update_working(vec![
            FileAction::overwrite("sub/hello.txt", "Hello world (from upstream)"),
        ]);
        upstream.add(".").unwrap();
        upstream.commit("First Commit from Upstream").unwrap();

        upstream.update_working(vec![
            FileAction::overwrite("hello.txt", "Not Applicable"),
        ]);
        upstream.add(".").unwrap();
        upstream.commit("Second Commit from Upstream").unwrap();

        upstream.push().unwrap();
    }, "sub").unwrap();
}


#[test]
pub fn import_na_a_2_commits() {
    TestWrapper::new("import_na_a_2_commits", |upstream| {
        upstream.update_working(vec![
            FileAction::overwrite("hello.txt", "Hello world (from upstream) - not applicable"),
        ]);
        upstream.add(".").unwrap();
        upstream.commit("First Commit from Upstream").unwrap();

        upstream.update_working(vec![
            FileAction::overwrite("sub/hello.txt", "Applicable"),
        ]);
        upstream.add(".").unwrap();
        upstream.commit("Second Commit from Upstream").unwrap();

        upstream.push().unwrap();
    }, "sub").unwrap();
}

#[test]
pub fn import_na_commit() {
    TestWrapper::new("import_na_commit", |upstream| {
        upstream.update_working(vec![
            FileAction::overwrite("hello.txt", "Not applicable"),
        ]);
        upstream.add(".").unwrap();
        upstream.commit("First Commit from Upstream").unwrap();

        upstream.update_working(vec![
            FileAction::overwrite("again.txt", "Also Not Applicable"),
        ]);
        upstream.add(".").unwrap();
        upstream.commit("Second Commit from Upstream").unwrap();

        upstream.push().unwrap();
    }, "sub").unwrap();
}

#[test]
pub fn import_two_na_commits() {
    TestWrapper::new("import_two_na_commits", |upstream| {
        upstream.update_working(vec![
            FileAction::overwrite("hello.txt", "Not applicable"),
        ]);
        upstream.add(".").unwrap();
        upstream.commit("First Commit from Upstream").unwrap();

        upstream.update_working(vec![
            FileAction::overwrite("again.txt", "Also Not Applicable"),
        ]);
        upstream.add(".").unwrap();
        upstream.commit("Second Commit from Upstream").unwrap();

        upstream.push().unwrap();
    }, "sub").unwrap();
}

#[test]
pub fn import_na_and_orphaned_a() {
    let test = TestWrapper::new("import_na_and_orphaned_a", |upstream| {
        upstream.update_working(vec![
            FileAction::overwrite("hello.txt", "Not applicable"),
        ]);
        upstream.add(".").unwrap();
        upstream.commit("First Commit from Upstream").unwrap();
        upstream.push().unwrap();

        upstream.checkout_adv(["--orphan", "orphaned"]).unwrap();
        upstream.update_working(vec![
            FileAction::remove("hello.txt"),
            FileAction::overwrite("sub/again.txt", "Applicable"),
        ]);
        upstream.add(".").unwrap();
        upstream.commit("Second Commit from Upstream").unwrap();

        upstream.push_adv(["-u", "origin", "orphaned"]).unwrap();

        upstream.checkout("master").unwrap();
    }, "sub").unwrap();

    test.do_then_verify(|u, s| {
        u.checkout("orphaned")?;
        s.checkout("orphaned")?;

        Ok(())
    });
}


#[test]
pub fn import_na_and_orphaned_na() {
    TestWrapper::new("import_na_and_orphaned_na", |upstream| {
        upstream.update_working(vec![
            FileAction::overwrite("hello.txt", "Not applicable"),
        ]);
        upstream.add(".").unwrap();
        upstream.commit("First Commit from Upstream").unwrap();
        upstream.push().unwrap();

        upstream.checkout_adv(["--orphan", "orphaned"]).unwrap();
        upstream.update_working(vec![
            FileAction::remove("hello.txt"),
            FileAction::overwrite("again.txt", "Also Not Applicable"),
        ]);
        upstream.add(".").unwrap();
        upstream.commit("Second Commit from Upstream").unwrap();

        upstream.push_adv(["-u", "origin", "orphaned"]).unwrap();
    }, "sub").unwrap();
}

#[test]
pub fn import_merged_na_na() {
    TestWrapper::new("import_merged_na_na", |upstream| {
        upstream.update_working(vec![
            FileAction::overwrite("hello.txt", "Not applicable"),
        ]);
        upstream.add(".").unwrap();
        upstream.commit("First Commit from Upstream").unwrap();

        upstream.checkout_adv(["--orphan", "orphaned"]).unwrap();
        upstream.update_working(vec![
            FileAction::remove("hello.txt"),
            FileAction::overwrite("again.txt", "Also Not Applicable"),
        ]);
        upstream.add(".").unwrap();
        upstream.commit("Second Commit from Upstream").unwrap();

        upstream.checkout("master").unwrap();
        upstream.merge(["orphaned"]).unwrap();
        upstream.push().unwrap();
    }, "sub").unwrap();
}

#[test]
pub fn import_merged_a_na() {
    let test = TestWrapper::new("import_merged_a_na", |upstream| {
        upstream.update_working(vec![
            FileAction::overwrite("sub/hello.txt", "Applicable"),
        ]);
        upstream.add(".").unwrap();
        upstream.commit("First Commit from Upstream").unwrap();

        upstream.checkout_adv(["--orphan", "orphaned"]).unwrap();
        upstream.update_working(vec![
            FileAction::remove("sub/hello.txt"),
            FileAction::overwrite("again.txt", "Not Applicable"),
        ]);
        upstream.add(".").unwrap();
        upstream.commit("Second Commit from Upstream").unwrap();

        upstream.checkout("master").unwrap();
        upstream.merge(["orphaned"]).unwrap();
        upstream.push().unwrap();
    }, "sub").unwrap();

    assert_eq!(test.get_subgit().commit_count("master").unwrap(), 2);
}


#[test]
pub fn import_merged_na_a() {
    let test = TestWrapper::new("import_merged_na_a", |upstream| {
        upstream.update_working(vec![
            FileAction::overwrite("hello.txt", "Not applicable"),
        ]);
        upstream.add(".").unwrap();
        upstream.commit("First Commit from Upstream").unwrap();

        upstream.checkout_adv(["--orphan", "orphaned"]).unwrap();
        upstream.update_working(vec![
            FileAction::remove("hello.txt"),
            FileAction::overwrite("sub/again.txt", "Applicable"),
        ]);
        upstream.add(".").unwrap();
        upstream.commit("Second Commit from Upstream").unwrap();

        upstream.checkout("master").unwrap();
        upstream.merge(["orphaned"]).unwrap();
        upstream.push().unwrap();
    }, "sub").unwrap();

    assert_eq!(test.get_subgit().commit_count("master").unwrap(), 2);
}

#[test]
pub fn import_merged_a_a() {
    TestWrapper::new("import_merged_a_a", |upstream| {
        upstream.update_working(vec![
            FileAction::overwrite("sub/hello.txt", "Not applicable"),
        ]);
        upstream.add(".").unwrap();
        upstream.commit("First Commit from Upstream").unwrap();

        upstream.checkout_adv(["--orphan", "orphaned"]).unwrap();
        upstream.update_working(vec![
            FileAction::remove("sub/hello.txt"),
            FileAction::overwrite("sub/again.txt", "Also Not Applicable"),
        ]);
        upstream.add(".").unwrap();
        upstream.commit("Second Commit from Upstream").unwrap();

        upstream.checkout("master").unwrap();
        upstream.merge(["orphaned"]).unwrap();
        upstream.push().unwrap();
    }, "sub").unwrap();
}
