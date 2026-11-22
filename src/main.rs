use std::process::ExitCode;

use sextant::shell::{parse_args, run, usage_text, CliError};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match parse_args(&args) {
        Ok(command) => ExitCode::from(run(command)),
        Err(CliError { message }) => {
            eprintln!("error: {}", message);
            eprintln!("{}", usage_text());
            ExitCode::from(2)
        }
    }
}
