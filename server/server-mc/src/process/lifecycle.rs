use std::{process::Stdio, time::Duration};

use anyhow::{anyhow, Result};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader, Lines},
    process::{Child, ChildStdin, ChildStdout, Command},
};
use tracing::info;

use super::RUN_DIR;

pub(super) struct ProcessLifeCycle {
    child: Child,
    stdin: ChildStdin,
    stdout: Lines<BufReader<ChildStdout>>,
}

impl ProcessLifeCycle {
    pub fn start(jar_path: &str) -> Result<Self> {
        let mut child = Command::new("java")
            .arg("-jar")
            .arg(jar_path)
            .arg("nogui")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .env_clear()
            .envs(std::env::vars().filter(|(k, _)| !k.starts_with("RUST")))
            .current_dir(RUN_DIR)
            .kill_on_drop(true)
            .spawn()?;
        let stdout = child
            .stdout
            .take()
            .ok_or(anyhow!("stdout is not available"))?;
        let stdin = child
            .stdin
            .take()
            .ok_or(anyhow!("stdin is not available"))?;

        Ok(Self {
            child,
            stdin,
            stdout: BufReader::new(stdout).lines(),
        })
    }

    pub async fn read_line(&mut self) -> Result<Option<String>> {
        let line = self.stdout.next_line().await?;
        if let Some(line) = &line {
            info!("mc: {line}");
        }
        Ok(line)
    }

    pub async fn write_command(&mut self, line: &str) -> Result<()> {
        self.stdin.write_all(line.as_bytes()).await?;
        self.stdin.write_all("\n".as_bytes()).await?;
        Ok(())
    }

    pub async fn stop_service(mut self) -> Result<()> {
        self.write_command("stop").await?;

        let wait_exit = self.child.wait();
        let max_to_wait = tokio::time::sleep(Duration::from_secs(10));
        let exited = tokio::select! {
            _ = wait_exit => true,
            _ = max_to_wait => false,
        };

        if !exited {
            self.child.kill().await?;
        }

        Ok(())
    }
}
