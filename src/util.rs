use libc;
use std::error;
use std::fmt;

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

/// See https://stackoverflow.com/questions/41494166/git-post-receive-hook-not-running-in-background
pub fn fork_into_child() {
    unsafe {
        libc::daemon(1, 0);
    }
}
