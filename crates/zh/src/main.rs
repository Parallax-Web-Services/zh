mod http;
mod resources;
mod shell;

use resources::Monitor;
use std::{env, ffi::OsString, io, process::{self, Command, ExitStatus, Stdio}, sync::Arc};

const HELP: &str = r#"Ж / zh 0.1.0-alpha — Distributed Private Cloud

Usage:
  zh                    Open Chromium resource map + persistent Bash session
  zh --headless         Bash shell without graphical interface
  zh --map              Resource map only (Ctrl+C to stop)
  zh --resources        Print local resource data as JSON
  zh -c 'command'      Run a Bash command
  zh script.sh [args]   Run an existing Bash script
  zh --help             Show this help

Environment:
  PARALLAX_CHROMIUM     Full path to your Parallax/Chromium browser binary
  ZH_BASH              Bash executable (default: bash)

Current limitations:
  Linux/WSL2 only; no peer scheduling, no remote shell, no in-browser PTY yet.
  *.zh / ZhQL workflows and zzh are not implemented in this alpha.
"#;

fn main() {
    if let Err(error) = run() {
        eprintln!("zh: {error}");
        process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    if !cfg!(target_os = "linux") {
        return Err("zh alpha currently supports Linux and WSL2 only".into());
    }

    let args: Vec<OsString> = env::args_os().skip(1).collect();
    let first = args.first().and_then(|s| s.to_str()).unwrap_or("");
    match first {
        "--help" | "-h" => { print!("{HELP}"); Ok(()) }
        "--version" | "-V" => { println!("zh 0.1.0-alpha"); Ok(()) }
        "--resources" => {
            let monitor = Monitor::new();
            // CPU delta requires two reads, not a percentage since boot.
            monitor.snapshot();
            std::thread::sleep(std::time::Duration::from_millis(250));
            println!("{}", serde_json::to_string_pretty(&monitor.snapshot())?);
            Ok(())
        }
        "--headless" => exit_like(shell::interactive()?),
        "-c" => {
            let command = args.get(1).ok_or("zh -c requires a command")?.clone();
            exit_like(shell::run_command(command, &args[2..])?)
        }
        "--map" => {
            let map = http::start(Arc::new(Monitor::new()))?;
            open_browser(&map.url);
            eprintln!("zh: resource map running at {} (Ctrl+C to stop)", map.url);
            loop { std::thread::park(); }
        }
        "" => {
            let map = http::start(Arc::new(Monitor::new()))?;
            open_browser(&map.url);
            eprintln!("zh: resource map: {}", map.url);
            eprintln!("zh: Bash compatibility mode; type 'exit' to close this session.");
            exit_like(shell::interactive()?)
        }
        other if other.ends_with(".zh") => {
            Err("ZhQL workflow execution is planned but not implemented yet".into())
        }
        other if other.ends_with(".sh") || other.ends_with(".bash") => {
            exit_like(shell::run_script(args[0].clone(), &args[1..])?)
        }
        _ => Err("unsupported argument; run 'zh --help'".into()),
    }
}

fn exit_like(status: ExitStatus) -> Result<(), Box<dyn std::error::Error>> {
    if status.success() { return Ok(()); }
    process::exit(status.code().unwrap_or(128));
}

fn open_browser(url: &str) {
    if let Ok(path) = env::var("PARALLAX_CHROMIUM") {
        if !path.is_empty() {
            match Command::new(path).arg(format!("--app={url}")).stdin(Stdio::null())
                .stdout(Stdio::null()).stderr(Stdio::null()).spawn() {
                Ok(_) => return,
                Err(error) => eprintln!("zh: PARALLAX_CHROMIUM failed: {error}"),
            }
        }
    }

    // A WSL2 installation can open the Windows browser through wslview.
    for browser in ["chromium", "chromium-browser", "google-chrome", "google-chrome-stable", "microsoft-edge"] {
        if Command::new(browser).arg(format!("--app={url}")).stdin(Stdio::null())
            .stdout(Stdio::null()).stderr(Stdio::null()).spawn().is_ok() { return; }
    }
    for helper in ["wslview", "xdg-open"] {
        if Command::new(helper).arg(url).stdin(Stdio::null())
            .stdout(Stdio::null()).stderr(Stdio::null()).spawn().is_ok() { return; }
    }
    eprintln!("zh: no browser launcher found; open this URL manually: {url}");
}
