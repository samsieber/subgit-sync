mod model;

#[cfg(test)]
mod tests {
    use super::model::*;

    #[test]
    fn can_build_tree() {
        // Build tree
        let target = ChangeSetGenerator::new("subgit/");

        let (initial, a5) = {
            let mut tree = CommitTree::new();
            let root = tree.root("root", target.empty());
            let a1 = tree.commit("a1", target.upstream(), root);
            let a2 = tree.commit("a2", target.subgit(), a1);
            let a3 = tree.commit("a3", target.both(), a1);
            let a4 = tree.merge_2("a4", a2, a3);
            let a5 = tree.commit("a5", target.upstream(), a4);

            tree.branch("master", a1);
            tree.branch("extra", a4);

            (tree, a5)
        };

        eprintln!("{:#?}", initial);

        let mut expected = initial.clone();
        let a6 = expected.commit("a6", target.both(), a5);
        let a7 = expected.commit("a7", target.both(), a6);
        let master = expected.branch("master", a7);

        eprintln!("{:#?}", expected)??        // Execute changes
//        // Setup test executor
//
//        let test_setup = register_test(test_name);
//        let test = test_setup.initial_tree(initial);
//
//        // Execute actions
//        let up1 = test.new_upstream_consumer("up1");
//        let up2 = test.new_upstream_consumer("up2");
//
//        up1
//            .commit(a6)
//            .push();
//        up2
//            .pull()
//            .commit(a7)
//            .push_fail()
//            .pull_merge()
//            .push();
//
//        // Run comparison
//        test.verify_trees(target);
    }
}
