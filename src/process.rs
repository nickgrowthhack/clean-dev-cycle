use crate::Result;
use std::{
    io::{Read, Write},
    process::{Child, Command, Output, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::{Duration, Instant},
};

pub fn capture(
    command: &mut Command,
    input: Vec<u8>,
    timeout: Duration,
    limit: usize,
    cancelled: &AtomicBool,
) -> Result<Output> {
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| {
            format!(
                "não foi possível executar {}: {error}.",
                command.get_program().to_string_lossy()
            )
        })?;
    let mut stdin = child.stdin.take().unwrap();
    let writer = thread::spawn(move || stdin.write_all(&input));
    let overflow = Arc::new(AtomicBool::new(false));
    let stdout = read_stream(child.stdout.take().unwrap(), limit, Arc::clone(&overflow));
    let stderr = read_stream(child.stderr.take().unwrap(), limit, Arc::clone(&overflow));
    let start = Instant::now();
    let mut exit_status = None;
    let status = loop {
        let reason = if cancelled.load(Ordering::Relaxed) {
            Some("operação cancelada.")
        } else if start.elapsed() >= timeout {
            Some("tempo limite excedido; nenhum commit foi solicitado ao Git.")
        } else if overflow.load(Ordering::Relaxed) {
            Some("a saída do processo excedeu o limite permitido.")
        } else {
            None
        };
        if let Some(reason) = reason {
            terminate(&mut child);
            return Err(reason.to_owned());
        }
        match child.try_wait() {
            Ok(Some(status)) => exit_status = Some(status),
            Ok(None) => {}
            Err(error) => {
                terminate(&mut child);
                return Err(format!("não foi possível acompanhar o processo: {error}."));
            }
        }
        if let Some(status) = exit_status
            && writer.is_finished()
            && stdout.is_finished()
            && stderr.is_finished()
        {
            break status;
        }
        thread::sleep(Duration::from_millis(30));
    };
    let write_result = writer
        .join()
        .map_err(|_| "falha ao enviar dados ao processo.")?;
    let stdout = stdout
        .join()
        .map_err(|_| "falha ao ler a saída do processo.")?
        .map_err(|e| format!("não foi possível ler a saída: {e}."))?;
    let stderr = stderr
        .join()
        .map_err(|_| "falha ao ler os erros do processo.")?
        .map_err(|e| format!("não foi possível ler os erros: {e}."))?;
    if overflow.load(Ordering::Relaxed) {
        return Err("a saída do processo excedeu o limite permitido.".into());
    }
    if status.success() {
        write_result.map_err(|e| format!("não foi possível enviar os dados completos: {e}."))?;
    }
    Ok(Output {
        status,
        stdout,
        stderr,
    })
}

fn read_stream(
    mut stream: impl Read + Send + 'static,
    limit: usize,
    overflow: Arc<AtomicBool>,
) -> thread::JoinHandle<std::io::Result<Vec<u8>>> {
    thread::spawn(move || {
        let mut bytes = Vec::new();
        let mut buffer = [0; 8192];
        loop {
            let count = stream.read(&mut buffer)?;
            if count == 0 {
                return Ok(bytes);
            }
            let available = limit.saturating_sub(bytes.len());
            bytes.extend_from_slice(&buffer[..count.min(available)]);
            if count > available {
                overflow.store(true, Ordering::Relaxed);
            }
        }
    })
}

fn terminate(child: &mut Child) {
    #[cfg(unix)]
    {
        let _ = Command::new("/bin/kill")
            .args(["-KILL", "--", &format!("-{}", child.id())])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
    #[cfg(windows)]
    {
        let _ = Command::new("taskkill.exe")
            .args(["/PID", &child.id().to_string(), "/T", "/F"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
    let _ = child.kill();
    let _ = child.wait();
}

pub fn diagnostic(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes)
        .chars()
        .filter(|c| !c.is_control() || *c == '\n' || *c == '\t')
        .take(2000)
        .collect::<String>()
        .trim()
        .to_owned()
}
