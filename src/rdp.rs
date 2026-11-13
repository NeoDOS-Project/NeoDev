//! `neodev rdp`: connect to a running VM's VirtualBox VRDE endpoint with a
//! local RDP client.
//!
//! This command is meant to run on the machine that owns the display (the
//! laptop/workstation), not on the headless VM host. It uses the same
//! `sdl-freerdp` binary bundled with the Remmina Flatpak that is known to send
//! the scancodes NeoDOS expects.

use crate::config::Config;
use anyhow::{Context, Result};
use colored::*;
use std::path::PathBuf;
use std::process::{Command, Stdio};

/// Options collected from the `rdp` subcommand.
pub struct RdpOptions {
    pub host: Option<String>,
    pub port: Option<u16>,
    pub user: String,
    pub password: String,
    pub width: u32,
    pub height: u32,
    pub client: String,
    pub print: bool,
}

struct Client {
    program: String,
    /// Arguments that precede the FreeRDP options (e.g. `flatpak run ...`).
    prefix: Vec<String>,
    label: String,
}

pub fn run(cfg: &Config, opts: &RdpOptions) -> Result<()> {
    let host = connect_host(cfg, opts.host.as_deref());
    let port = opts.port.unwrap_or(cfg.vbox_vrde_port);
    let client = resolve_client(&opts.client)?;
    let args = freerdp_args(
        &host,
        port,
        &opts.user,
        &opts.password,
        opts.width,
        opts.height,
    );

    println!(
        "{} RDP endpoint: {}:{} (client: {})",
        "[*]".bold().cyan(),
        host,
        port,
        client.label
    );
    if !cfg.vbox_vrde && opts.host.is_none() {
        eprintln!(
            "{} [virtualbox] vrde is disabled; make sure the VM is running with VRDE enabled.",
            "[!]".bold().yellow()
        );
    }

    if opts.print {
        println!("{}", render_command(&client, &args));
        return Ok(());
    }

    if std::env::var_os("DISPLAY").is_none() && std::env::var_os("WAYLAND_DISPLAY").is_none() {
        anyhow::bail!(
            "neodev rdp needs a graphical session (DISPLAY or WAYLAND_DISPLAY).\n  \
             Run it on the machine with the display (for example your laptop), not on the \
             headless VM host. Use --print to only print the client command."
        );
    }

    println!("{} Launching {}...", "[*]".bold().cyan(), client.label);
    let status = Command::new(&client.program)
        .args(&client.prefix)
        .args(&args)
        .status()
        .with_context(|| format!("Failed to launch RDP client '{}'", client.program))?;
    if !status.success() {
        anyhow::bail!("RDP client '{}' exited with {}", client.program, status);
    }
    Ok(())
}

/// Resolve the host to connect to. A wildcard VRDE bind address means "this
/// machine", so map it to loopback; an explicit address is used as-is.
fn connect_host(cfg: &Config, cli_host: Option<&str>) -> String {
    if let Some(host) = cli_host {
        return host.to_string();
    }
    let bind = cfg.vbox_vrde_address.trim();
    if bind.is_empty() || matches!(bind, "0.0.0.0" | "::" | "[::]" | "*") {
        "127.0.0.1".into()
    } else {
        bind.to_string()
    }
}

fn freerdp_args(
    host: &str,
    port: u16,
    user: &str,
    password: &str,
    width: u32,
    height: u32,
) -> Vec<String> {
    vec![
        format!("/v:{}:{}", host, port),
        format!("/u:{}", user),
        format!("/p:{}", password),
        "/cert:ignore".into(),
        format!("/w:{}", width),
        format!("/h:{}", height),
    ]
}

fn resolve_client(requested: &str) -> Result<Client> {
    match requested.to_ascii_lowercase().as_str() {
        "auto" => {
            if let Some(program) = which("sdl-freerdp") {
                return Ok(Client {
                    program,
                    prefix: Vec::new(),
                    label: "sdl-freerdp".into(),
                });
            }
            if flatpak_remmina_available() {
                return Ok(flatpak_remmina_client());
            }
            if let Some(program) = which("xfreerdp") {
                return Ok(Client {
                    program,
                    prefix: Vec::new(),
                    label: "xfreerdp".into(),
                });
            }
            anyhow::bail!(
                "No RDP client found. Install sdl-freerdp (recommended for correct NeoDOS \
                 scancodes), xfreerdp, or the Remmina Flatpak:\n  \
                 flatpak install flathub org.remmina.Remmina"
            );
        }
        "sdl-freerdp" => Ok(Client {
            program: require_client("sdl-freerdp", "sdl-freerdp")?,
            prefix: Vec::new(),
            label: "sdl-freerdp".into(),
        }),
        "xfreerdp" => Ok(Client {
            program: require_client("xfreerdp", "xfreerdp")?,
            prefix: Vec::new(),
            label: "xfreerdp".into(),
        }),
        "remmina" | "remmina-flatpak" | "flatpak" => {
            if !flatpak_remmina_available() {
                anyhow::bail!(
                    "The Remmina Flatpak is not installed. Install it with:\n  \
                     flatpak install flathub org.remmina.Remmina"
                );
            }
            Ok(flatpak_remmina_client())
        }
        // Any other value is treated as an explicit command path/name.
        _ => Ok(Client {
            program: requested.to_string(),
            prefix: Vec::new(),
            label: requested.to_string(),
        }),
    }
}

fn flatpak_remmina_client() -> Client {
    Client {
        program: "flatpak".into(),
        prefix: vec![
            "run".into(),
            "--command=sdl-freerdp".into(),
            "org.remmina.Remmina".into(),
        ],
        label: "Remmina (sdl-freerdp Flatpak)".into(),
    }
}

fn require_client(command: &str, label: &str) -> Result<String> {
    which(command).ok_or_else(|| {
        anyhow::anyhow!(
            "RDP client '{}' not found in PATH. Install {} or use --client remmina-flatpak.",
            label,
            label
        )
    })
}

fn flatpak_remmina_available() -> bool {
    Command::new("flatpak")
        .args(["info", "org.remmina.Remmina"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn which(command: &str) -> Option<String> {
    let paths = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&paths) {
        let candidate: PathBuf = dir.join(command);
        if candidate.is_file() {
            return Some(candidate.to_string_lossy().into_owned());
        }
    }
    None
}

fn render_command(client: &Client, args: &[String]) -> String {
    let mut parts = Vec::with_capacity(1 + client.prefix.len() + args.len());
    parts.push(shell_quote(&client.program));
    parts.extend(client.prefix.iter().map(|arg| shell_quote(arg)));
    parts.extend(args.iter().map(|arg| shell_quote(arg)));
    parts.join(" ")
}

fn shell_quote(arg: &str) -> String {
    if !arg.is_empty()
        && arg.chars().all(|c| {
            c.is_ascii_alphanumeric() || matches!(c, '/' | ':' | '.' | '_' | '-' | '=' | ',')
        })
    {
        arg.to_string()
    } else {
        format!("'{}'", arg.replace('\'', "'\\''"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn config() -> Config {
        Config::default(Path::new("/tmp/neodos"))
    }

    #[test]
    fn wildcard_vrde_bind_maps_to_loopback() {
        let mut cfg = config();
        cfg.vbox_vrde_address = "0.0.0.0".into();
        assert_eq!(connect_host(&cfg, None), "127.0.0.1");
        cfg.vbox_vrde_address = "::".into();
        assert_eq!(connect_host(&cfg, None), "127.0.0.1");
    }

    #[test]
    fn explicit_hosts_win() {
        let mut cfg = config();
        cfg.vbox_vrde_address = "10.0.1.10".into();
        assert_eq!(connect_host(&cfg, None), "10.0.1.10");
        assert_eq!(connect_host(&cfg, Some("192.0.2.5")), "192.0.2.5");
    }

    #[test]
    fn freerdp_arguments_match_the_known_good_launcher() {
        let args = freerdp_args("10.0.1.10", 5000, "neodos", ".", 1024, 768);
        assert_eq!(
            args,
            vec![
                "/v:10.0.1.10:5000",
                "/u:neodos",
                "/p:.",
                "/cert:ignore",
                "/w:1024",
                "/h:768",
            ]
        );
    }

    #[test]
    fn rendered_flatpak_command_is_runnable() {
        let client = flatpak_remmina_client();
        let args = freerdp_args("10.0.1.10", 5000, "neodos", ".", 1024, 768);
        let rendered = render_command(&client, &args);
        assert!(rendered.starts_with(
            "flatpak run --command=sdl-freerdp org.remmina.Remmina /v:10.0.1.10:5000"
        ));
    }

    #[test]
    fn shell_quoting_handles_spaces_and_quotes() {
        assert_eq!(shell_quote("plain"), "plain");
        assert_eq!(shell_quote("with space"), "'with space'");
        assert_eq!(shell_quote("it's"), "'it'\\''s'");
    }
}
