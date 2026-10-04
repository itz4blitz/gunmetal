//! The `gunmetal` binary: hands the process's arguments, environment and
//! standard streams to the command line in the library, and exits with the
//! code it answers.

use std::ffi::OsString;
use std::io;
use std::process::ExitCode;

use gunmetal_server::cli;

fn main() -> ExitCode {
    let args: Vec<OsString> = std::env::args_os().skip(1).collect();
    ExitCode::from(run(&args, std::env::vars_os().collect()))
}

/// Runs the command line `args` in the environment `vars` and returns its
/// exit code.
fn run(args: &[OsString], vars: Vec<(OsString, OsString)>) -> u8 {
    cli::run(args, vars, Box::new(io::stdout()), &mut io::stderr()).code()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_command_line_without_a_subcommand_exits_with_the_usage_code() {
        assert_eq!(run(&[], Vec::new()), 64);
    }
}
