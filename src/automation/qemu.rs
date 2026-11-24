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
        stream.set_read_timeout(Some(Duration::from_secs(5))).ok();

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
    let cleaned: Vec<u8> = buf.iter().copied().filter(|&b| b != b'\r').collect();
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

/// Build the HMP `sendkey` payloads for a string.
///
/// One command per character: QEMU treats keys joined with `-` as a chord, so
/// sending a whole line at once keeps `shift` held after a shifted key and the
/// following characters arrive shifted (`:` then `\` types `|`, `:` then `/`
/// types `?`).
fn key_commands(keys: &str) -> Vec<String> {
    keys.chars()
        .filter_map(|c| {
            let seq = char_to_qemu_keys(c);
            if seq.is_empty() {
                None
            } else {
                Some(seq.join("-"))
            }
        })
        .collect()
}

impl super::VmAutomation for QemuAutomation {
    fn send_keys(&self, keys: &str) -> Result<()> {
        for keys in key_commands(keys) {
            self.send_hmp(&format!("sendkey {keys}"))?;
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

#[cfg(test)]
mod tests {
    use super::{char_to_qemu_keys, key_commands};

    #[test]
    fn key_commands_are_one_per_character() {
        // The regression from #52: `:` used to share a chord with the next
        // key, leaving shift held so `\` arrived as `|`.
        assert_eq!(
            key_commands("C:\\dir"),
            ["shift-c", "shift-semicolon", "backslash", "d", "i", "r"]
        );
        assert_eq!(key_commands("/?"), ["slash", "shift-slash"]);
        assert_eq!(key_commands("a b"), ["a", "spc", "b"]);
    }

    #[test]
    fn key_commands_never_join_symbols() {
        for c in "\\/:;?|_+<>[]{}~`".chars() {
            let seq = char_to_qemu_keys(c);
            assert!(!seq.is_empty(), "missing mapping for {c:?}");
            let cmd = key_commands(&c.to_string());
            assert_eq!(cmd.len(), 1, "one command for {c:?}");
            assert_eq!(cmd[0], seq.join("-"));
        }
    }

    #[test]
    fn unmapped_characters_are_skipped() {
        assert!(key_commands("\u{1F600}").is_empty());
    }
}
