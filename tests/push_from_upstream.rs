extern crate subgit_rs;
extern crate log;
extern crate simplelog;

mod harness;
mod util;

use harness::*;
use std::time::Duration;

fn base(name: &str) -> TestWrapper {
    TestWrapper::new(name, |upstream| {
        upstream.update_working(vec![
            FileAction::overwrite("sub/hello.txt", "Hello world (from upstream)"),
        ]);
        upstream.add(".").unwrap();
        upstream.commit("First Commit from Upstream").unwrap();
        upstream.push().unwrap();
    }, "sub").unwrap()
}


#[test]
pub fn push_and_then_delete_branch() {
    let test = base("push_and_then_delete_branch");

    test.do_then_verify(|upstream, downstream| {
        upstream.checkout_adv(["-b", "second"]).unwrap();
        upstream.update_working(vec![FileAction::overwrite("second.txt", "sec")]);
        upstream.add(".").unwrap();
        upstream.commit("Commit from second branch").unwrap();
        upstream.push_adv(["origin", "second:second"]).unwrap();

        std::thread::sleep(Duration::new(2,0));

        downstream.pull().unwrap();
        downstream.checkout("second").unwrap();

        Ok(())
    });

    test.do_then_verify(|upstream, downstream| {
        upstream.push_adv(["origin", ":second"]).unwrap();
        assert_eq!("", upstream.command_output(vec!["ls-remote", "--heads", "origin", "second"]).unwrap());

        std::thread::sleep(Duration::new(2,0));

        downstream.command_output(vec!["fetch", "--all"]).unwrap();
        assert_eq!("", downstream.command_output(vec!["ls-remote", "--heads", "origin", "second"]).unwrap());

        Ok(())
    });
}
