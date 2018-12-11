use declarative::*;

#[test]
pub fn na_a_2_commits() {
    let config : TestConfig = config_for!("na_a_2_commits");
    let (target, executor) = target_executor();

    let tree = {
        let mut tree = CommitTree::new();
        let root = tree.root("root", target.empty());
        let a1 = tree.commit("a1", target.upstream(), &root);
        let a2 = tree.commit("a2", target.subgit(), &a1);
        let a3 = tree.commit("a3", target.both(), &a1);
        let a4 = tree.merge_2("a4", &a2, &a3);
        let a5 = tree.commit("a5", target.upstream(), &a4);

        tree.branch("master", &a4);
        tree.branch("extra", &a5);

        tree
    };

    config.run_setup(tree.clone(), executor).verify(tree);
}
