use anyhow::{Context, Result};
use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

static HMP_PROMPT: &[u8] = b"(qemu) ";

pub struct QemuAutomation {
    port: u16,
}

impl QemuAutomation {
    pub fn new(port: u16) -> Result<Self> {
        // Verify the monitor is reachable
        let mut stream = TcpStream::connect_timeout(
            &format!("127.0.0.1:{}", port).parse().unwrap(),
            Duration::from_secs(2),
        )
        .context(format!(
            "Cannot connect to QEMU automation monitor on port {}. Is the VM running?",
            port
        ))?;
        let _ = stream.set_read_timeout(Some(Duration::from_secs(3)));
        // Consume the banner and initial prompt
        let _ = read_until_prompt(&mut stream);
        drop(stream);
        Ok(Self { port })
    }

    fn send_hmp(&self, command: &str) -> Result<String> {
        let mut stream = TcpStream::connect_timeout(
            &format!("127.0.0.1:{}", self.port).parse().unwrap(),
            Duration::from_secs(3),
        )
        .context("Failed to connect to QEMU automation monitor")?;
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .ok();

        // Drain any stale prompt
        let _ = read_until_prompt(&mut stream);

        // Send command with trailing newline
        let cmd_bytes = format!("{}\n", command);
        stream
            .write_all(cmd_bytes.as_bytes())
            .context("Failed to write to QEMU monitor")?;
        stream
            .flush()
            .context("Failed to flush QEMU monitor stream")?;

        // Read response until next prompt
        let response = read_until_prompt(&mut stream)?;
        Ok(response)
    }
}

fn read_until_prompt(stream: &mut TcpStream) -> Result<String> {
    let mut buf: Vec<u8> = Vec::new();
    let mut tmp = [0u8; 4096];
    loop {
        match stream.read(&mut tmp) {
            Ok(0) => break,
            Ok(n) => {
                buf.extend_from_slice(&tmp[..n]);
                if buf.len() >= HMP_PROMPT.len() && buf.ends_with(HMP_PROMPT) {
                    break;
                }
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
            Err(e) => return Err(e.into()),
        }
    }
    // Strip carriage returns and filter telnet IAC (0xFF) sequences
    let cleaned: Vec<u8> = buf
        .iter()
        .copied()
        .filter(|&b| b != b'\r')
        .collect();
    Ok(String::from_utf8_lossy(&cleaned).to_string())
}

fn char_to_qemu_keys(c: char) -> Vec<String> {
    match c {
        'a'..='z' => vec![c.to_string()],
        'A'..='Z' => vec![format!("shift-{}", c.to_ascii_lowercase())],
        '0'..='9' => vec![c.to_string()],
        ' ' => vec!["spc".into()],
        '.' => vec!["dot".into()],
        '-' => vec!["minus".into()],
        '_' => vec!["shift-minus".into()],
        '/' => vec!["slash".into()],
        '\\' => vec!["backslash".into()],
        ':' => vec!["shift-semicolon".into()],
        ';' => vec!["semicolon".into()],
        '\'' => vec!["apostrophe".into()],
        '"' => vec!["shift-apostrophe".into()],
        '!' => vec!["shift-1".into()],
        '@' => vec!["shift-2".into()],
        '#' => vec!["shift-3".into()],
        '$' => vec!["shift-4".into()],
        '%' => vec!["shift-5".into()],
        '^' => vec!["shift-6".into()],
        '&' => vec!["shift-7".into()],
        '*' => vec!["shift-8".into()],
        '(' => vec!["shift-9".into()],
        ')' => vec!["shift-0".into()],
        '+' => vec!["shift-equal".into()],
        '=' => vec!["equal".into()],
        ',' => vec!["comma".into()],
        '<' => vec!["shift-comma".into()],
        '>' => vec!["shift-dot".into()],
        '?' => vec!["shift-slash".into()],
        '~' => vec!["shift-backquote".into()],
        '`' => vec!["backquote".into()],
        '|' => vec!["shift-backslash".into()],
        '{' => vec!["shift-lbracket".into()],
        '}' => vec!["shift-rbracket".into()],
        '[' => vec!["lbracket".into()],
        ']' => vec!["rbracket".into()],
        '\n' => vec!["ret".into()],
        '\t' => vec!["tab".into()],
        _ => vec![],
    }
}

impl super::VmAutomation for QemuAutomation {
    fn send_keys(&self, keys: &str) -> Result<()> {
        // Batch keys into commands of ~50 to avoid excessively long monitor lines
        let chars: Vec<char> = keys.chars().collect();
        for chunk in chars.chunks(50) {
            let key_seq: Vec<String> =
                chunk.iter().flat_map(|&c| char_to_qemu_keys(c)).collect();
            if key_seq.is_empty() {
                continue;
            }
            let cmd = format!("sendkey {}", key_seq.join("-"));
            self.send_hmp(&cmd)?;
        }
        Ok(())
    }

    fn send_enter(&self) -> Result<()> {
        self.send_hmp("sendkey ret")?;
        Ok(())
    }

    fn send_command(&self, cmd: &str) -> Result<()> {
        self.send_keys(cmd)?;
        self.send_enter()?;
        Ok(())
    }
}
