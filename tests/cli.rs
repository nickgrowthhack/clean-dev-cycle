use std::process::{Command, Output};

fn invoke(arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_clean-dev-cycle"))
        .args(arguments)
        .current_dir(std::env::temp_dir())
        .output()
        .expect("o binário deve executar sem depender do diretório do projeto")
}

#[test]
fn help_is_available_without_a_repository() {
    for arguments in [&[][..], &["--help"][..], &["-h"][..]] {
        let output = invoke(arguments);
        assert!(output.status.success());
        assert!(output.stderr.is_empty());
        let help = String::from_utf8(output.stdout).expect("ajuda em UTF-8");
        assert!(help.contains("Uso: clean-dev-cycle"));
        assert!(help.contains("Opções:"));
        assert!(help.contains("--version"));
    }
}

#[test]
fn version_matches_the_installable_package() {
    for argument in ["--version", "-V"] {
        let output = invoke(&[argument]);
        assert!(output.status.success());
        assert!(output.stderr.is_empty());
        assert_eq!(
            String::from_utf8(output.stdout)
                .expect("versão em UTF-8")
                .trim(),
            format!("clean-dev-cycle {}", env!("CARGO_PKG_VERSION"))
        );
    }
}

#[test]
fn invalid_invocations_cannot_report_success() {
    for arguments in [
        &["--inexistente"][..],
        &["ação-desconhecida"][..],
        &["--version", "extra"][..],
        &["--help", "--version"][..],
    ] {
        let output = invoke(arguments);
        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty());
        let error = String::from_utf8(output.stderr).expect("erro em UTF-8");
        assert!(error.contains("Erro:"));
        assert!(error.contains("--help"));
    }
}
