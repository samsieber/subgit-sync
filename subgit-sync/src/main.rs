extern crate subgit_sync;

use failure::Fail;

fn main() {
    std::env::set_var("RUST_BACKTRACE", "1");
    subgit_sync::run().expect("Failed to complete setup");
}
