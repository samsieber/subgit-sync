use std::error;
use std::fmt;
use std::path::Path;
use std::ffi::OsStr;
use std;
use std::error::Error;
use std::process::Output;


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

pub fn write_files<P, K,V,I>(root: P, files: I) -> Result<(), Box<Error>>
    where P: AsRef<Path>, K: AsRef<Path>, V: AsRef<[u8]>, I: IntoIterator<Item=(K,V)>
{
    let root_dir = root.as_ref();

    for (f, c) in files {
        std::fs::create_dir_all(root_dir.join(&f).parent().unwrap())?;
        std::fs::write(root_dir.join(&f), c)?;
    }

    Ok(())
}


pub fn command<P, C, I, S>(path: P, command: C, args: I) -> Result<(), Box<Error>>
    where P: AsRef<Path>, C: AsRef<OsStr>, I: IntoIterator<Item=S>, S: AsRef<OsStr>
{
    let result = command_raw(path, command, args)?;

    if !result.status.success() {
        let err_message =format!(
            "Could not execute command {}. Full command output: \nStd Out:\n{}\nStd Err:\n{}",
            &result.status,
            String::from_utf8(result.stdout)?,
            String::from_utf8(result.stderr)?
        );

        println!("{}", err_message);

        return Err(Box::new(StringError { message: err_message }));
    } else {
        println!("{}", String::from_utf8(result.stdout)?);
        println!("{}", String::from_utf8(result.stderr)?);
    }

    Ok(())
}

pub fn command_raw<P, C, I, S>(path: P, command: C, args: I) -> Result<Output, Box<Error>>
    where P: AsRef<Path>, C: AsRef<OsStr>, I: IntoIterator<Item=S>, S: AsRef<OsStr>
{
    let mut process = std::process::Command::new(&command);
    process
        .env_clear()
        .env("PATH", std::env::var("PATH").unwrap());

    process.args(args);
    process.current_dir(path.as_ref());

    Ok(process.output()?)
}

/// Double forks the current process, exiting the parent processes.
/// Additionally, closes the file descriptors so that this works over an ssh connection
/// Panics on failure
///
/// See https://stackoverflow.com/questions/41494166/git-post-receive-hook-not-running-in-background
/// Also see https://users.rust-lang.org/t/how-to-close-a-file-descriptor-with-nix/9878
pub fn fork_into_child() {
    match fork() {
        Ok(ForkResult::Parent { child, .. }) => {
            std::process::exit(0);
        }
        Ok(ForkResult::Child) => {
            match fork() {
                Ok(ForkResult::Parent { child, .. }) => {
                    std::process::exit(0);
                }
                Ok(ForkResult::Child) => {
                    {
                        unsafe {
                            File::from_raw_fd(0);
                            File::from_raw_fd(1);
                            File::from_raw_fd(2);
                        }
                    }
                },
                Err(_) => panic!("Second Fork failed"),
            }
        },
        Err(_) => panic!("Fork failed"),
    }
}