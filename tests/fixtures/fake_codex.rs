use std::{env, fs, io::{Read, Write}, path::PathBuf, process::Command, thread, time::Duration};

fn main() {
    let args: Vec<_> = env::args_os().skip(1).collect();
    assert!(args.iter().any(|arg| arg == "--ignore-user-config"));
    assert!(args.iter().any(|arg| arg == "--ephemeral"));
    assert!(args.iter().any(|arg| arg == "read-only"));
    assert!(args.windows(2).any(|pair| pair[0] == "--disable" && pair[1] == "shell_tool"));
    let repo = PathBuf::from(env::var_os("FAKE_REPO").unwrap());
    assert_ne!(env::current_dir().unwrap(), repo);
    let log = PathBuf::from(env::var_os("FAKE_LOG").unwrap());
    let mut input = String::new();
    std::io::stdin().read_to_string(&mut input).unwrap();
    let previous = fs::read_to_string(&log).unwrap_or_default();
    fs::write(&log, format!("{previous}call\n")).unwrap();
    fs::write(log.with_extension("prompt"), &input).unwrap();
    fs::write(log.with_extension("args"), format!("{args:?}")).unwrap();
    let mode = env::var("FAKE_MODE").unwrap();
    if mode == "timeout" { thread::sleep(Duration::from_secs(20)); }
    if mode == "failure" { eprintln!("falha simulada do provedor"); std::process::exit(7); }
    if mode == "overflow" { for _ in 0..2000 { println!("{}", "x".repeat(8192)); } return; }
    if mode == "mutate" {
        fs::write(repo.join("outra.txt"), "alteração concorrente").unwrap();
    }
    if mode == "mutate-base" {
        assert!(Command::new("git").current_dir(&repo).args(["update-ref", "refs/heads/main", "HEAD"]).status().unwrap().success());
    }
    let index = args.iter().position(|arg| arg == "--output-last-message").unwrap();
    let response = if mode == "needs-context" {
        r#"{"message":"","needs_context":true,"reason":"Qual é a intenção desta mudança?"}"#
    } else if mode == "invalid" || (mode == "retry" && previous.is_empty()) {
        r#"{"message":"mensagem sem tipo","needs_context":false,"reason":""}"#
    } else if mode == "malformed" {
        "não é JSON"
    } else if input.contains("Você revisa uma entrega completa") {
        r####"{"message":"### Revisão completa de entregas\n\nAs mudanças do intervalo recebem uma síntese do resultado acumulado.","needs_context":false,"reason":""}"####
    } else {
        r#"{"message":"feat(cli): implementar geração de commits","needs_context":false,"reason":""}"#
    };
    fs::write(&args[index + 1], response).unwrap();
    if mode == "tool" { println!(r#"{{"type":"item.completed","item":{{"type":"command_execution"}}}}"#); }
    else { println!(r#"{{"type":"item.completed","item":{{"type":"agent_message"}}}}"#); }
    std::io::stdout().flush().unwrap();
}
