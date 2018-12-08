extern crate subgit_sync;

fn main() {
    std::env::set_var("RUST_BACKTRACE", "1");
    subgit_sync::run().expect("Could not run properly");
}
