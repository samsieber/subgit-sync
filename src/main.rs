extern crate subgit_rs;

fn main() {
    std::env::set_var("RUST_BACKTRACE", "1");
    subgit_rs::run().expect("Could not run properly");
}
