use std::{
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
    process::{Command, ExitStatus, Stdio},
};

use anyhow::{Context, Result, anyhow, bail};

const CHROME_DEB_URL: &str =
    "https://dl.google.com/linux/direct/google-chrome-stable_current_amd64.deb";
const FIREFOX_TAR_URL: &str =
    "https://download.mozilla.org/?product=firefox-latest&os=linux64&lang=en-US";
const EDGE_PACKAGE_INDEX_URL: &str =
    "https://packages.microsoft.com/repos/edge/dists/stable/main/binary-amd64/Packages.gz";
const EDGE_PACKAGE_BASE_URL: &str = "https://packages.microsoft.com/repos/edge";

pub fn install_deps() -> Result<()> {
    let distro = Distro::detect()?;
    let need_headless = needs_headless_runtime();

    println!("Detected distro: {}", distro.label());
    println!(
        "Display available: {}",
        if need_headless { "no" } else { "yes" }
    );

    install_runtime_packages(&distro, need_headless)?;

    if prompt_yes_no("Install Chrome?", true)? {
        install_chrome_local()?;
    }
    if prompt_yes_no("Install Edge?", true)? {
        install_edge_local()?;
    }
    if prompt_yes_no("Install Firefox?", true)? {
        install_firefox_local()?;
    }
    if prompt_yes_no("Install Camoufox?", true)? {
        install_camoufox_local()?;
    }

    println!("install-deps complete");
    println!("Make sure ~/.local/bin is on PATH, then run: axonbrowser --help");
    Ok(())
}

fn install_runtime_packages(distro: &Distro, need_headless: bool) -> Result<()> {
    let mut packages = vec!["imagemagick", "xclip", "xdotool", "curl", "ca-certificates"];

    match distro.package_manager {
        PackageManager::Apt => {
            packages.extend(["at-spi2-core", "dbus-x11", "python3-venv", "x11-utils"]);
            if need_headless {
                packages.extend(["xvfb"]);
            }
            run_privileged("apt-get", &["update"])?;
            let mut args = vec!["install", "-y"];
            args.extend(packages.iter().copied());
            run_privileged("apt-get", &args)?;
        }
        PackageManager::Dnf => {
            packages.extend([
                "at-spi2-core",
                "dbus-x11",
                "python3",
                "python3-pip",
                "xorg-x11-utils",
            ]);
            if need_headless {
                packages.extend(["xorg-x11-server-Xvfb"]);
            }
            let mut args = vec!["install", "-y"];
            args.extend(packages.iter().copied());
            run_privileged("dnf", &args)?;
        }
        PackageManager::Pacman => {
            packages.extend([
                "at-spi2-core",
                "dbus",
                "python",
                "python-pip",
                "xorg-xdpyinfo",
            ]);
            if need_headless {
                packages.extend(["xorg-server-xvfb"]);
            }
            run_privileged("pacman", &["-Sy", "--noconfirm"])?;
            let mut args = vec!["-S", "--noconfirm"];
            args.extend(packages.iter().copied());
            run_privileged("pacman", &args)?;
        }
        PackageManager::Zypper => {
            packages.extend([
                "at-spi2-core",
                "dbus-1-x11",
                "python312",
                "python312-pip",
                "xprop",
            ]);
            if need_headless {
                packages.extend(["xorg-x11-server-extra"]);
            }
            let mut args = vec!["install", "-y"];
            args.extend(packages.iter().copied());
            run_privileged("zypper", &args)?;
        }
    }

    Ok(())
}

fn install_chrome_local() -> Result<()> {
    ensure_commands(["curl", "ar", "tar"])?;
    let workdir = temp_workdir("axonbrowser-chrome-install")?;
    let deb_path = workdir.join("google-chrome.deb");
    download_to(CHROME_DEB_URL, &deb_path)?;

    let unpack_dir = workdir.join("pkg");
    fs::create_dir_all(&unpack_dir)?;
    unpack_deb(&deb_path, &unpack_dir)?;

    let source_dir = unpack_dir.join("opt/google/chrome");
    let prefix = local_opt_dir().join("google-chrome");
    install_tree(&source_dir, &prefix)?;
    write_wrapper("google-chrome", &prefix.join("google-chrome"))?;
    println!("installed Chrome to {}", prefix.display());
    Ok(())
}

fn install_firefox_local() -> Result<()> {
    ensure_commands(["curl", "tar"])?;
    let workdir = temp_workdir("axonbrowser-firefox-install")?;
    let tar_path = workdir.join("firefox.tar.xz");
    download_to(FIREFOX_TAR_URL, &tar_path)?;

    let extract_dir = workdir.join("extract");
    fs::create_dir_all(&extract_dir)?;
    run_command(
        "tar",
        &[
            "-xf",
            &tar_path.display().to_string(),
            "-C",
            &extract_dir.display().to_string(),
        ],
    )?;

    let source_dir = extract_dir.join("firefox");
    let prefix = local_opt_dir().join("firefox");
    install_tree(&source_dir, &prefix)?;
    write_wrapper("firefox", &prefix.join("firefox"))?;
    println!("installed Firefox to {}", prefix.display());
    Ok(())
}

fn install_edge_local() -> Result<()> {
    ensure_commands(["curl", "gzip", "awk", "ar", "tar"])?;
    let filename = latest_edge_filename()?;
    let url = format!("{EDGE_PACKAGE_BASE_URL}/{filename}");

    let workdir = temp_workdir("axonbrowser-edge-install")?;
    let deb_path = workdir.join("microsoft-edge.deb");
    download_to(&url, &deb_path)?;

    let unpack_dir = workdir.join("pkg");
    fs::create_dir_all(&unpack_dir)?;
    unpack_deb(&deb_path, &unpack_dir)?;

    let source_dir = unpack_dir.join("opt/microsoft/msedge");
    let prefix = local_opt_dir().join("microsoft-edge");
    install_tree(&source_dir, &prefix)?;
    write_wrapper("microsoft-edge", &prefix.join("microsoft-edge"))?;
    println!("installed Edge to {}", prefix.display());
    Ok(())
}

fn install_camoufox_local() -> Result<()> {
    ensure_commands(["python3"])?;
    let venv_dir = local_share_dir().join("camoufox-venv");
    if !venv_dir.exists() {
        run_command("python3", &["-m", "venv", &venv_dir.display().to_string()])?;
    }

    let pip = venv_dir.join("bin/pip");
    let python = venv_dir.join("bin/python");
    run_command(
        pip.to_str().ok_or_else(|| anyhow!("invalid pip path"))?,
        &["install", "--upgrade", "pip"],
    )?;
    run_command(
        pip.to_str().ok_or_else(|| anyhow!("invalid pip path"))?,
        &["install", "camoufox"],
    )?;

    let bin = run_capture(
        python
            .to_str()
            .ok_or_else(|| anyhow!("invalid python path"))?,
        &["-m", "camoufox", "path"],
    )?;
    let resolved = bin.trim();
    if resolved.is_empty() {
        bail!("camoufox path command returned an empty path");
    }

    let wrapper_path = local_bin_dir().join("camoufox");
    fs::create_dir_all(local_bin_dir())?;
    fs::write(
        &wrapper_path,
        format!(
            "#!/usr/bin/env bash\nset -euo pipefail\nBIN=\"{}\"\nif [[ -d \"$BIN\" ]]; then\n  BIN=\"$BIN/camoufox\"\nfi\nexec \"$BIN\" \"$@\"\n",
            resolved
        ),
    )?;
    make_executable(&wrapper_path)?;
    println!("installed Camoufox in {}", venv_dir.display());
    Ok(())
}

fn latest_edge_filename() -> Result<String> {
    let output = run_capture(
        "sh",
        &[
            "-lc",
            &format!(
                "curl -fsSL {url} | gzip -dc | awk '/^Package: microsoft-edge-stable$/ {{ pkg=1; next }} pkg && /^Filename:/ {{ latest=$2; pkg=0 }} END {{ print latest }}'",
                url = EDGE_PACKAGE_INDEX_URL
            ),
        ],
    )?;
    let filename = output.trim();
    if filename.is_empty() {
        bail!("failed to resolve latest Edge package filename");
    }
    Ok(filename.to_string())
}

fn unpack_deb(deb_path: &Path, unpack_dir: &Path) -> Result<()> {
    run_command_in(
        "ar",
        &[
            "x",
            deb_path
                .to_str()
                .ok_or_else(|| anyhow!("invalid deb path"))?,
        ],
        Some(unpack_dir),
    )?;

    let data_xz = unpack_dir.join("data.tar.xz");
    let data_gz = unpack_dir.join("data.tar.gz");
    let archive = if data_xz.exists() {
        data_xz
    } else if data_gz.exists() {
        data_gz
    } else {
        bail!("deb archive did not contain data.tar.xz or data.tar.gz");
    };

    run_command_in(
        "tar",
        &[
            "-xf",
            archive
                .to_str()
                .ok_or_else(|| anyhow!("invalid archive path"))?,
        ],
        Some(unpack_dir),
    )
}

fn install_tree(source: &Path, dest: &Path) -> Result<()> {
    if !source.exists() {
        bail!("source install tree missing: {}", source.display());
    }
    if dest.exists() {
        fs::remove_dir_all(dest).with_context(|| format!("failed to remove {}", dest.display()))?;
    }
    fs::create_dir_all(dest.parent().unwrap_or_else(|| Path::new("/")))?;
    run_command_in(
        "cp",
        &[
            "-a",
            &format!("{}/.", source.display()),
            &dest.display().to_string(),
        ],
        None::<&Path>,
    )
}

fn write_wrapper(name: &str, target: &Path) -> Result<()> {
    fs::create_dir_all(local_bin_dir())?;
    let wrapper_path = local_bin_dir().join(name);
    fs::write(
        &wrapper_path,
        format!(
            "#!/usr/bin/env bash\nset -euo pipefail\nexec \"{}\" \"$@\"\n",
            target.display()
        ),
    )?;
    make_executable(&wrapper_path)?;
    Ok(())
}

fn make_executable(path: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(path)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(path, perms)?;
    }
    Ok(())
}

fn download_to(url: &str, dest: &Path) -> Result<()> {
    run_command(
        "curl",
        &[
            "-fsSL",
            url,
            "-o",
            dest.to_str()
                .ok_or_else(|| anyhow!("invalid download path"))?,
        ],
    )
}

fn run_privileged(program: &str, args: &[&str]) -> Result<()> {
    let use_sudo = !is_root();
    let mut command = if use_sudo {
        if !command_exists("sudo") {
            bail!(
                "{} requires root privileges and sudo is not available",
                program
            );
        }
        let mut cmd = Command::new("sudo");
        cmd.arg(program);
        cmd
    } else {
        Command::new(program)
    };

    let status = command
        .args(args)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .with_context(|| format!("failed to run {}", program))?;
    ensure_success(program, status)
}

fn run_command(program: &str, args: &[&str]) -> Result<()> {
    run_command_in(program, args, None::<&Path>)
}

fn run_command_in<P: AsRef<Path>>(program: &str, args: &[&str], workdir: Option<P>) -> Result<()> {
    let mut command = Command::new(program);
    command.args(args);
    if let Some(workdir) = workdir {
        command.current_dir(workdir);
    }
    let status = command
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .with_context(|| format!("failed to run {}", program))?;
    ensure_success(program, status)
}

fn run_capture(program: &str, args: &[&str]) -> Result<String> {
    let output = Command::new(program)
        .args(args)
        .output()
        .with_context(|| format!("failed to run {}", program))?;
    if !output.status.success() {
        bail!(
            "{} failed: {}",
            program,
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

fn ensure_success(program: &str, status: ExitStatus) -> Result<()> {
    if status.success() {
        Ok(())
    } else {
        bail!("{} exited with status {}", program, status)
    }
}

fn ensure_commands<const N: usize>(commands: [&str; N]) -> Result<()> {
    let missing = commands
        .into_iter()
        .filter(|command| !command_exists(command))
        .collect::<Vec<_>>();
    if missing.is_empty() {
        Ok(())
    } else {
        bail!("missing required commands: {}", missing.join(", "))
    }
}

fn command_exists(program: &str) -> bool {
    Command::new("sh")
        .args(["-lc", &format!("command -v {}", program)])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn prompt_yes_no(prompt: &str, default_yes: bool) -> Result<bool> {
    loop {
        let suffix = if default_yes { "[Y/n]" } else { "[y/N]" };
        print!("{} {} ", prompt, suffix);
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let answer = input.trim().to_ascii_lowercase();
        if answer.is_empty() {
            return Ok(default_yes);
        }
        match answer.as_str() {
            "y" | "yes" => return Ok(true),
            "n" | "no" => return Ok(false),
            _ => {
                println!("Please answer yes or no.");
            }
        }
    }
}

fn needs_headless_runtime() -> bool {
    let display = std::env::var("DISPLAY").ok();
    let Some(display) = display else {
        return true;
    };

    !Command::new("xdpyinfo")
        .arg("-display")
        .arg(display)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn is_root() -> bool {
    Command::new("id")
        .arg("-u")
        .output()
        .ok()
        .map(|output| String::from_utf8_lossy(&output.stdout).trim() == "0")
        .unwrap_or(false)
}

fn temp_workdir(prefix: &str) -> Result<PathBuf> {
    let dir = std::env::temp_dir().join(format!("{prefix}-{}", std::process::id()));
    if dir.exists() {
        fs::remove_dir_all(&dir).ok();
    }
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

fn local_bin_dir() -> PathBuf {
    PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| ".".to_string())).join(".local/bin")
}

fn local_opt_dir() -> PathBuf {
    PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| ".".to_string()))
        .join(".local/opt/axonbrowser")
}

fn local_share_dir() -> PathBuf {
    PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| ".".to_string()))
        .join(".local/share/axonbrowser")
}

struct Distro {
    id: String,
    like: String,
    package_manager: PackageManager,
}

impl Distro {
    fn detect() -> Result<Self> {
        let content = fs::read_to_string("/etc/os-release")
            .context("failed to read /etc/os-release for distro detection")?;
        let mut id = String::new();
        let mut like = String::new();
        for line in content.lines() {
            if let Some(value) = line.strip_prefix("ID=") {
                id = trim_os_release(value);
            } else if let Some(value) = line.strip_prefix("ID_LIKE=") {
                like = trim_os_release(value);
            }
        }

        let package_manager = if command_exists("apt-get") {
            PackageManager::Apt
        } else if command_exists("dnf") {
            PackageManager::Dnf
        } else if command_exists("pacman") {
            PackageManager::Pacman
        } else if command_exists("zypper") {
            PackageManager::Zypper
        } else {
            bail!("unsupported Linux distro: no supported package manager found")
        };

        Ok(Self {
            id,
            like,
            package_manager,
        })
    }

    fn label(&self) -> String {
        if self.like.is_empty() {
            self.id.clone()
        } else {
            format!("{} ({})", self.id, self.like)
        }
    }
}

#[derive(Clone, Copy)]
enum PackageManager {
    Apt,
    Dnf,
    Pacman,
    Zypper,
}

fn trim_os_release(value: &str) -> String {
    value.trim().trim_matches('"').to_string()
}
