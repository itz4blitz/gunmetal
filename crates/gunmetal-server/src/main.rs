//! The `gunmetal` binary: hands the process's arguments, environment and
//! standard streams to the command line in the library, and exits with the
//! code it answers.

use std::io;
use std::process::ExitCode;

use gunmetal_server::cli;

fn main() -> ExitCode {
    ExitCode::from(run())
}

/// Runs the command line this process was started with and returns its
/// exit code.
fn run() -> u8 {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    cli::run(
        &args,
        std::env::vars_os().collect(),
        Box::new(io::stdout()),
        &mut io::stderr(),
    )
    .code()
}

#[cfg(test)]
mod tests {
    use super::*;

    // The test harness's own command line names no gunmetal subcommand, so
    // running it is refused as a usage error.
    #[test]
    fn a_command_line_without_a_subcommand_exits_with_the_usage_code() {
        assert_eq!(run(), 64);
        assert_eq!(format!("{:?}", main()), format!("{:?}", ExitCode::from(64)));
    }
}
