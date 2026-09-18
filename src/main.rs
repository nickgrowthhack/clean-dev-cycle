mod changelog;
mod check_commit;
mod ci;
mod cli;
mod commit;
mod executable;
mod git;
mod message;
mod process;
mod provider;

use std::process::ExitCode;

type Result<T> = std::result::Result<T, String>;

fn main() -> ExitCode {
    let arguments: Vec<_> = std::env::args_os().skip(1).collect();
    if arguments
        .first()
        .is_some_and(|value| value == "__verify-commit")
    {
        return finish(commit::verify_hook(&arguments[1..]));
    }
    match cli::parse(arguments) {
        Ok(cli::Action::Help(help)) => {
            print!("{help}");
            ExitCode::SUCCESS
        }
        Ok(cli::Action::Version) => {
            println!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        Ok(cli::Action::Commit(options)) => finish(commit::run(options)),
        Ok(cli::Action::CheckCommit(options)) => finish(check_commit::run(options)),
        Ok(cli::Action::CheckCi(options)) => finish(ci::run(options)),
        Ok(cli::Action::Changelog(options)) => finish(changelog::run(options)),
        Err(error) => {
            eprintln!("Erro: {error}\nUse clean-dev-cycle --help para consultar o uso.");
            ExitCode::from(2)
        }
    }
}

fn finish(result: Result<()>) -> ExitCode {
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("Erro: {error}");
            ExitCode::FAILURE
        }
    }
}
