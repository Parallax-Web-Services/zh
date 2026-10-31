//! Bash compatibility is provided by an actual persistent Bash process in alpha.
//! ZhQL is a separate native workflow feature (not yet implemented).

use std::{ffi::OsString, io, process::{Command, ExitStatus}};

fn executable() -> String {
    std::env::var("ZH_BASH").unwrap_or_else(|_| "bash".to_owned())
}

pub fn interactive() -> io::Result<ExitStatus> {
    Command::new(executable()).arg("-i").status()
}

pub fn run_script(script: OsString, args: &[OsString]) -> io::Result<ExitStatus> {
    Command::new(executable()).arg(script).args(args).status()
}

pub fn run_command(command: OsString, args: &[OsString]) -> io::Result<ExitStatus> {
    Command::new(executable()).arg("-c").arg(command).args(args).status()
}
