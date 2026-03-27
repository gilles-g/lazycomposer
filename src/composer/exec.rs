use std::io::BufRead;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread;

/// Strips carriage returns from PTY output while preserving ANSI color codes.
fn strip_cr(s: &str) -> String {
    s.replace('\r', "")
}

/// StreamLine carries either a line of output or a final signal.
#[derive(Debug)]
pub struct StreamLine {
    pub text: String,
    pub err: Option<String>,
    pub done: bool,
}

/// StreamHandle holds a stream receiver and the child PID for kill support.
pub struct StreamHandle {
    pub rx: mpsc::Receiver<StreamLine>,
    pub child_pid: Option<u32>,
}

/// Kills a process by PID using SIGTERM.
pub fn kill_process(pid: u32) {
    let _ = std::process::Command::new("kill")
        .args(["-TERM", &pid.to_string()])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();
}

/// RunResult holds the output of a synchronous command.
pub struct RunResult {
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub exit_code: i32,
}

/// Executor abstracts running composer CLI commands for testability.
pub trait Executor: Send + Sync {
    fn run(&self, dir: &str, args: &[&str]) -> Result<RunResult, String>;
    fn stream(&self, dir: &str, args: &[&str]) -> Result<StreamHandle, String>;
    fn bin(&self) -> String;
    fn look_path(&self) -> String;
}

/// RealExecutor runs actual composer commands via std::process::Command.
pub struct RealExecutor {
    cmd: String,
    prefix: Vec<String>,
}

impl RealExecutor {
    pub fn new(bin: &str) -> Self {
        let parts = split_command(bin);
        let cmd = parts[0].clone();
        let prefix = parts[1..].to_vec();
        RealExecutor { cmd, prefix }
    }

    fn build_args(&self, args: &[&str]) -> Vec<String> {
        let mut result: Vec<String> = self.prefix.clone();
        result.extend(args.iter().map(|s| s.to_string()));
        result
    }
}

impl Executor for RealExecutor {
    fn run(&self, dir: &str, args: &[&str]) -> Result<RunResult, String> {
        let full_args = self.build_args(args);
        let output = Command::new(&self.cmd)
            .args(&full_args)
            .current_dir(dir)
            .output()
            .map_err(|e| e.to_string())?;

        let exit_code = output.status.code().unwrap_or(-1);

        Ok(RunResult {
            stdout: output.stdout,
            stderr: output.stderr,
            exit_code,
        })
    }

    fn stream(&self, dir: &str, args: &[&str]) -> Result<StreamHandle, String> {
        let full_args = self.build_args(args);

        // Use `script` to allocate a PTY so composer flushes output in real-time.
        // `script -qefc "cmd" /dev/null` merges stdout+stderr through a PTY.
        let inner_cmd = std::iter::once(self.cmd.as_str())
            .chain(full_args.iter().map(|s| s.as_str()))
            .map(shell_escape)
            .collect::<Vec<_>>()
            .join(" ");

        let mut child = Command::new("script")
            .args(["-qefc", &inner_cmd, "/dev/null"])
            .current_dir(dir)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| e.to_string())?;

        let child_pid = child.id();
        let stdout = child.stdout.take().ok_or("failed to capture stdout")?;

        let (tx, rx) = mpsc::channel();

        thread::spawn(move || {
            let reader = std::io::BufReader::new(stdout);
            for line in reader.lines() {
                match line {
                    Ok(text) => {
                        let clean = strip_cr(&text);
                        if clean.is_empty() {
                            continue;
                        }
                        let _ = tx.send(StreamLine {
                            text: clean,
                            err: None,
                            done: false,
                        });
                    }
                    Err(e) => {
                        let _ = tx.send(StreamLine {
                            text: String::new(),
                            err: Some(e.to_string()),
                            done: true,
                        });
                        return;
                    }
                }
            }

            let status = child.wait();
            match status {
                Ok(s) if s.success() || s.code() == Some(1) => {
                    let _ = tx.send(StreamLine {
                        text: String::new(),
                        err: None,
                        done: true,
                    });
                }
                Ok(s) => {
                    let _ = tx.send(StreamLine {
                        text: String::new(),
                        err: Some(format!("exit code {}", s.code().unwrap_or(-1))),
                        done: true,
                    });
                }
                Err(e) => {
                    let _ = tx.send(StreamLine {
                        text: String::new(),
                        err: Some(e.to_string()),
                        done: true,
                    });
                }
            }
        });

        Ok(StreamHandle {
            rx,
            child_pid: Some(child_pid),
        })
    }

    fn bin(&self) -> String {
        let mut parts = vec![self.cmd.clone()];
        parts.extend(self.prefix.clone());
        parts.join(" ")
    }

    fn look_path(&self) -> String {
        which::which(&self.cmd)
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| self.cmd.clone())
    }
}

/// Escapes a string for safe use in a shell command.
fn shell_escape(s: &str) -> String {
    if s.chars()
        .all(|c| c.is_alphanumeric() || "-_./=:@^".contains(c))
    {
        s.to_string()
    } else {
        format!("'{}'", s.replace('\'', "'\\''"))
    }
}

/// Splits a command string into parts, respecting quoted strings.
pub fn split_command(s: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut in_quote: Option<char> = None;

    for c in s.chars() {
        match in_quote {
            Some(q) => {
                if c == q {
                    in_quote = None;
                } else {
                    current.push(c);
                }
            }
            None => match c {
                '\'' | '"' => {
                    in_quote = Some(c);
                }
                ' ' | '\t' => {
                    if !current.is_empty() {
                        parts.push(current.clone());
                        current.clear();
                    }
                }
                _ => {
                    current.push(c);
                }
            },
        }
    }

    if !current.is_empty() {
        parts.push(current);
    }

    if parts.is_empty() {
        vec!["composer".to_string()]
    } else {
        parts
    }
}
