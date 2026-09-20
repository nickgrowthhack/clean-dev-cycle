mod changelog;
mod check_commit;
mod checks;
mod cli;
mod commit;
mod executable;
mod git;
mod github;
mod jj;
mod message;
mod process;
mod provider;
mod release;
mod submit;

use std::process::ExitCode;

type Result<T> = std::result::Result<T, String>;

fn main() -> ExitCode {
    let arguments: Vec<_> = std::env::args_os().skip(1).collect();
    match cli::parse(arguments) {
        Ok(cli::Action::Help(help)) => {
            print!("{help}");
            ExitCode::SUCCESS
        }
        Ok(cli::Action::Version) => {
            println!(
                "{} {}",
                env!("CARGO_PKG_NAME"),
                env!("CLEAN_DEV_CYCLE_VERSION")
            );
            ExitCode::SUCCESS
        }
        Ok(cli::Action::Commit(options)) => finish(commit::run(options)),
        Ok(cli::Action::CheckCommit(options)) => finish(check_commit::run(options)),
        Ok(cli::Action::Submit(revision)) => finish(submit::run(&revision)),
        Ok(cli::Action::Changelog(options)) => finish(changelog::run(options)),
        Ok(cli::Action::Release {
            publish,
            revision,
            assets,
        }) => finish(release::run(&revision, publish, &assets)),
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
