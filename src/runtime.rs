use std::{
    collections::BTreeMap,
    fs::{self, File},
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

use anyhow::{Context, Result, anyhow, bail};

const DISPLAY_CANDIDATES: std::ops::RangeInclusive<u16> = 99..=110;
const XVFB_SCREEN: &str = "1440x900x24";
const WAIT_TIMEOUT: Duration = Duration::from_secs(10);
const WAIT_POLL: Duration = Duration::from_millis(200);

pub fn bootstrap_headless_session() -> Result<()> {
    if has_working_display(None) {
        return Ok(());
    }

    let session = match load_persisted_session()? {
        Some(session) => session,
        None => start_session()?,
    };
    apply_env(&session.env);
    Ok(())
}

struct HeadlessSession {
    env: BTreeMap<String, String>,
}

fn start_session() -> Result<HeadlessSession> {
    ensure_commands([
        "Xvfb",
        "xdpyinfo",
        "dbus-launch",
        "openbox",
        "x11vnc",
    ])?;

    let dir = session_dir()?;
    let display_number = choose_display_number()?;
    let display = format!(":{display_number}");

    let xvfb_log = dir.join("xvfb.log");
    let openbox_log = dir.join("openbox.log");
    let vnc_log = dir.join("x11vnc.log");

    let xvfb_pid = spawn_logged(
        "Xvfb",
        &[
            &display,
            "-screen",
            "0",
            XVFB_SCREEN,
            "-ac",
            "-nolisten",
            "tcp",
        ],
        &xvfb_log,
        None,
    )?;
    write_pid_file(&dir.join("xvfb.pid"), xvfb_pid)?;
    wait_for_display(&display)?;

    let mut env = base_session_env(&display);
    let dbus_env = launch_dbus_session()?;
    env.extend(dbus_env);

    let openbox_pid = spawn_logged("openbox", &[], &openbox_log, Some(&env))?;
    write_pid_file(&dir.join("openbox.pid"), openbox_pid)?;

    let vnc_port = 5900u16 + display_number;
    let vnc_port_text = vnc_port.to_string();
    let x11vnc_pid = spawn_logged(
        "x11vnc",
        &[
            "-display",
            &display,
            "-forever",
            "-shared",
            "-localhost",
            "-nopw",
            "-rfbport",
            &vnc_port_text,
        ],
        &vnc_log,
        Some(&env),
    )?;
    write_pid_file(&dir.join("x11vnc.pid"), x11vnc_pid)?;

    env.insert("GUIBOT_HEADLESS".to_string(), "1".to_string());
    env.insert(
        "GUIBOT_HEADLESS_SESSION_DIR".to_string(),
        dir.display().to_string(),
    );
    env.insert("GUIBOT_VNC_PORT".to_string(), vnc_port_text);

    persist_env(&dir.join("session.env"), &env)?;

    Ok(HeadlessSession { env })
}

fn load_persisted_session() -> Result<Option<HeadlessSession>> {
    let env_path = session_dir()?.join("session.env");
    if !env_path.exists() {
        return Ok(None);
    }

    let env = parse_env_file(&env_path)?;
    if env.is_empty() {
        return Ok(None);
    }

    if has_working_display(Some(&env)) {
        return Ok(Some(HeadlessSession { env }));
    }

    Ok(None)
}

fn session_dir() -> Result<PathBuf> {
    let home = std::env::var("HOME").context("HOME is not set")?;
    let dir = PathBuf::from(home).join(".cache/axonbrowser/headless");
    fs::create_dir_all(&dir)
        .with_context(|| format!("failed to create session dir {}", dir.display()))?;
    Ok(dir)
}

fn ensure_commands<const N: usize>(commands: [&str; N]) -> Result<()> {
    let missing = commands
        .into_iter()
        .filter(|command| !command_exists(command))
        .collect::<Vec<_>>();
    if missing.is_empty() {
        return Ok(());
    }

    bail!(
        "missing headless runtime commands: {}",
        missing.join(", ")
    )
}

fn command_exists(command: &str) -> bool {
    Command::new("sh")
        .args(["-lc", &format!("command -v {command}")])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn has_working_display(env_override: Option<&BTreeMap<String, String>>) -> bool {
    let display = env_override
        .and_then(|env| env.get("DISPLAY"))
        .cloned()
        .or_else(|| std::env::var("DISPLAY").ok());
    let Some(display) = display else {
        return false;
    };

    let mut command = Command::new("xdpyinfo");
    command.arg("-display").arg(display);
    if let Some(env) = env_override {
        command.env_clear();
        command.envs(env);
        command.env("PATH", std::env::var("PATH").unwrap_or_default());
    }

    command
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn choose_display_number() -> Result<u16> {
    for display in DISPLAY_CANDIDATES {
        let socket = PathBuf::from(format!("/tmp/.X11-unix/X{display}"));
        if !socket.exists() {
            return Ok(display);
        }
    }

    Err(anyhow!(
        "could not find a free X11 display in {:?}",
        DISPLAY_CANDIDATES
    ))
}

fn wait_for_display(display: &str) -> Result<()> {
    let start = Instant::now();
    while start.elapsed() < WAIT_TIMEOUT {
        let status = Command::new("xdpyinfo")
            .arg("-display")
            .arg(display)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
        if status.map(|s| s.success()).unwrap_or(false) {
            return Ok(());
        }
        thread::sleep(WAIT_POLL);
    }

    bail!("timed out waiting for X11 display {display} to become ready")
}

fn launch_dbus_session() -> Result<BTreeMap<String, String>> {
    let output = Command::new("dbus-launch")
        .arg("--sh-syntax")
        .output()
        .context("failed to launch dbus session")?;
    if !output.status.success() {
        bail!(
            "dbus-launch failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut env = BTreeMap::new();
    for line in stdout.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with("echo ") {
            continue;
        }
        let Some((key, rest)) = trimmed.split_once('=') else {
            continue;
        };
        let value = rest.split(';').next().unwrap_or_default().trim();
        env.insert(key.to_string(), strip_shell_quotes(value));
    }

    if !env.contains_key("DBUS_SESSION_BUS_ADDRESS") {
        bail!("dbus-launch did not return DBUS_SESSION_BUS_ADDRESS");
    }
    Ok(env)
}

fn strip_shell_quotes(value: &str) -> String {
    value
        .trim()
        .trim_matches('\'')
        .trim_matches('"')
        .to_string()
}

fn base_session_env(display: &str) -> BTreeMap<String, String> {
    let mut env = BTreeMap::new();
    env.insert("DISPLAY".to_string(), display.to_string());
    env.insert("XDG_SESSION_TYPE".to_string(), "x11".to_string());
    env.insert("NO_AT_BRIDGE".to_string(), "0".to_string());
    env.insert("GTK_MODULES".to_string(), "gail:atk-bridge".to_string());
    env.insert(
        "QT_LINUX_ACCESSIBILITY_ALWAYS_ON".to_string(),
        "1".to_string(),
    );
    env.insert("ACCESSIBILITY_ENABLED".to_string(), "1".to_string());
    env.insert("GNOME_ACCESSIBILITY".to_string(), "1".to_string());
    if let Ok(home) = std::env::var("HOME") {
        env.insert("HOME".to_string(), home);
    }
    env
}

fn apply_env(env: &BTreeMap<String, String>) {
    for (key, value) in env {
        unsafe {
            std::env::set_var(key, value);
        }
    }
}

fn spawn_logged(
    command: &str,
    args: &[&str],
    log_path: &Path,
    env_override: Option<&BTreeMap<String, String>>,
) -> Result<u32> {
    let log_path = log_path.display().to_string();
    let command = shell_escape(command);
    let args = args
        .iter()
        .map(|arg| shell_escape(arg))
        .collect::<Vec<_>>()
        .join(" ");
    let shell_script = format!(
        "nohup {command}{space}{args} >>{log} 2>&1 </dev/null & echo $!",
        space = if args.is_empty() { "" } else { " " },
        log = shell_escape(&log_path),
    );

    let mut cmd = Command::new("sh");
    cmd.args(["-lc", &shell_script])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    if let Some(env) = env_override {
        cmd.envs(env);
    }

    let output = cmd
        .spawn()
        .with_context(|| format!("failed to start background command {command}"))?
        .wait_with_output()
        .with_context(|| format!("failed waiting for background command shell {command}"))?;
    let pid = String::from_utf8_lossy(&output.stdout)
        .trim()
        .parse::<u32>()
        .with_context(|| format!("failed to parse background pid for {command}"))?;
    Ok(pid)
}

fn shell_escape(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\"'\"'"))
}

fn persist_env(path: &Path, env: &BTreeMap<String, String>) -> Result<()> {
    let mut file = File::create(path)
        .with_context(|| format!("failed to create session env file {}", path.display()))?;
    for (key, value) in env {
        writeln!(file, "{key}={value}")
            .with_context(|| format!("failed writing session env file {}", path.display()))?;
    }
    Ok(())
}

fn parse_env_file(path: &Path) -> Result<BTreeMap<String, String>> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("failed to read session env file {}", path.display()))?;
    let mut env = BTreeMap::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let Some((key, value)) = trimmed.split_once('=') else {
            continue;
        };
        env.insert(key.trim().to_string(), value.trim().to_string());
    }
    Ok(env)
}

fn write_pid_file(path: &Path, pid: u32) -> Result<()> {
    fs::write(path, pid.to_string())
        .with_context(|| format!("failed to write pid file {}", path.display()))
}
