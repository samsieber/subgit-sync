#[macro_use]
extern crate log;
extern crate subgit_rs;

fn main() {
    subgit_rs::setup_logging();
    info!("Hello, world!");
}
