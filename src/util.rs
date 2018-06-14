use std::error;
use std::fmt;
use std;
use libc;


use std::os::unix::io::FromRawFd;
use nix::unistd::{fork, ForkResult};
use std::fs::File;

#[derive(Debug)]
pub struct StringError {
    pub message: String,
}

impl fmt::Display for StringError {
    // This trait requires `fmt` with this exact signature.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // Write strictly the first element into the supplied output
        // stream: `f`. Returns `fmt::Result` which indicates whether the
        // operation succeeded or failed. Note that `write!` uses syntax which
        // is very similar to `println!`.
        write!(f, "{}", self.message)
    }
}

impl error::Error for StringError {
    fn description(&self) -> &str {
        self.message.as_ref()
    }

    fn cause(&self) -> Option<&error::Error> {
        None
    }
}

/// Double forks the current process, exiting the parent processes.
/// Additionally, closes the file descriptors so that this works over an ssh connection
/// Panics on failure
///
/// See https://stackoverflow.com/questions/41494166/git-post-receive-hook-not-running-in-background
/// Also see https://users.rust-lang.org/t/how-to-close-a-file-descriptor-with-nix/9878
pub fn fork_into_child() {
    match fork() {
        Ok(ForkResult::Parent { child: _child, .. }) => {
            std::process::exit(0);
        }
        Ok(ForkResult::Child) => {
            match fork() {
                Ok(ForkResult::Parent { child: _child, .. }) => {
                    std::process::exit(0);
                }
                Ok(ForkResult::Child) => {
                    {
                        unsafe{
                            libc::daemon(1, 0);
                        }
                    }
                },
                Err(_) => panic!("Second Fork failed"),
            }
        },
        Err(_) => panic!("Fork failed"),
    }
}