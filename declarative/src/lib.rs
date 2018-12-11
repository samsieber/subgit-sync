mod util;
mod tree;

mod harness;

mod executor;
mod git;

pub use crate::tree::{CommitTree,ChangeSetGenerator};
pub use crate::executor::DefaultExecutor;
pub use crate::git::GitConsumer;
pub use crate::harness::{TestConfig, Test};

#[macro_export]
macro_rules! config_for {
    ($test_name:expr) => {
        TestConfig::new($test_name, module_path!())
    }
}

pub fn executor() -> DefaultExecutor {
    DefaultExecutor {
        log_file: Some("setup.log".to_owned()),
        log_level: None,
        use_daemon: true
    }
}

pub fn target_executor() -> (ChangeSetGenerator, DefaultExecutor) {
    (ChangeSetGenerator::new("subgit/"), executor())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn can_build_tree() {
        // Build tree
        let target = ChangeSetGenerator::new("subgit/");

        let (initial, a4) = {
            let mut tree = CommitTree::new();
            let root = tree.root("root", target.empty());
            let a1 = tree.commit("a1", target.upstream(), &root);
            let a2 = tree.commit("a2", target.subgit(), &a1);
            let a3 = tree.commit("a3", target.both(), &a1);
            let a4 = tree.merge_2("a4", &a2, &a3);
            let a5 = tree.commit("a5", target.upstream(), &a4);

            tree.branch("master", &a4);
            tree.branch("extra", &a5);

            (tree, a4)
        };

        eprintln!("{:#?}", initial);

        let mut expected = initial.clone();
        let a6 = expected.commit("a6", target.both(), &a4);
        let a7 = expected.commit("a7", target.both(), &a6);
        let a8 = expected.merge_2("a8", &a7, &a6);

        expected.branch("master", &a8);

        eprintln!("{:#?}", expected);        // Execute changes
        // Setup test executor

        let config = config_for!("My basic test");

        eprintln!("{:#?}", config);

        let test = config.run_setup(initial.clone(), DefaultExecutor {
            log_file: Some("setup.log".to_owned()),
            log_level: None,
            use_daemon: true
        });


        // Execute actions
        let up1 = test.upstream_consumer("up1");
        let up2 = test.upstream_consumer("up2");

        up1
            .commit(a6)
            .push("master");
        up2
            .commit(a7)
            .push_fail("master")
            .pull_merge()
            .push("master");

        crate::util::sleep(2);

        // Run comparison
        test.verify(expected);
    }
}
