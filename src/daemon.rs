use crate::{serve, BootArgs, Result};
use daemonize::Daemonize;
use std::{
    fs::{File, Permissions},
    os::unix::fs::PermissionsExt,
    path::Path,
};

#[cfg(target_family = "unix")]
const PID_PATH: &str = "/var/run/fs.pid";
#[cfg(target_family = "unix")]
const DEFAULT_STDOUT_PATH: &str = "/var/run/fs.out";
#[cfg(target_family = "unix")]
const DEFAULT_STDERR_PATH: &str = "/var/run/fs.err";

/// Get the pid of the daemon
#[cfg(target_family = "unix")]
pub(crate) fn get_pid() -> Option<i32> {
    if let Ok(data) = std::fs::read(PID_PATH) {
        let binding = String::from_utf8(data).expect("pid file is not utf8");
        return Some(binding.trim().parse().expect("pid file is not a number"));
    }
    None
}

/// Check if the current user is root
#[cfg(target_family = "unix")]
pub fn check_root() {
    if !nix::unistd::Uid::effective().is_root() {
        println!("You must run this executable with root permissions");
        std::process::exit(-1)
    }
}

/// Run the server
pub fn run(args: BootArgs) -> Result<()> {
    serve::run(args)
}

/// Start the daemon
#[cfg(target_family = "unix")]
pub fn start(args: BootArgs) -> Result<()> {
    use crate::homedir::setting_dir;

    if let Some(pid) = get_pid() {
        println!("fs is already running with pid: {}", pid);
        return Ok(());
    }

    check_root();

    let pid_file = File::create(PID_PATH)?;
    pid_file.set_permissions(Permissions::from_mode(0o755))?;

    let stdout = File::create(DEFAULT_STDOUT_PATH)?;
    stdout.set_permissions(Permissions::from_mode(0o755))?;

    let stderr = File::create(DEFAULT_STDERR_PATH)?;
    stdout.set_permissions(Permissions::from_mode(0o755))?;

    let mut daemonize = Daemonize::new()
        .pid_file(PID_PATH) // Every method except `new` and `start`
        .chown_pid_file(true) // is optional, see `Daemonize` documentation
        .umask(0o777) // Set umask, `0o027` by default.
        .stdout(stdout) // Redirect stdout to `/tmp/daemon.out`.
        .stderr(stderr) // Redirect stderr to `/tmp/daemon.err`.
        .privileged_action(|| "Executed before drop privileges");

    if let Ok(user) = std::env::var("SUDO_USER") {
        if let Ok(Some(real_user)) = nix::unistd::User::from_name(&user) {
            #[cfg(not(target_os = "windows"))]
            setting_dir(real_user.dir);
            daemonize = daemonize
                .user(real_user.name.as_str())
                .group(real_user.gid.as_raw());
        }
    }

    if let Some(err) = daemonize.start().err() {
        eprintln!("Error: {err}");
        std::process::exit(-1)
    }

    run(args)
}

/// Stop the daemon
#[cfg(target_family = "unix")]
pub fn stop() -> Result<()> {
    use nix::{sys::signal, unistd::Pid};

    check_root();

    if let Some(pid) = get_pid() {
        for _ in 0..360 {
            if signal::kill(Pid::from_raw(pid), signal::SIGINT).is_err() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_secs(1))
        }
        let _ = std::fs::remove_file(PID_PATH);
    }

    Ok(())
}

/// Restart the daemon
#[cfg(target_family = "unix")]
pub fn restart(args: BootArgs) -> Result<()> {
    stop()?;
    start(args)
}

/// Show the status of the daemon
#[cfg(target_family = "unix")]
pub fn status() -> Result<()> {
    use sysinfo::System;

    match get_pid() {
        Some(pid) => {
            let mut sys = System::new();

            // First we update all information of our `System` struct.
            sys.refresh_all();

            // Display processes ID
            let process = sys
                .processes()
                .iter()
                .find(|(raw_pid, _)| raw_pid.as_u32().eq(&(pid as u32)))
                .ok_or_else(|| crate::error::Error::NotRunning)?;

            println!("{:<6} {:<6}  {:<6}", "PID", "CPU(%)", "MEM(MB)");
            println!(
                "{:<6}   {:<6.1}  {:<6.1}",
                process.0,
                process.1.cpu_usage(),
                (process.1.memory() as f64) / 1024.0 / 1024.0
            );
        }
        None => println!("fs is not running"),
    }
    Ok(())
}

/// Show the log of the daemon
#[cfg(target_family = "unix")]
pub fn log() -> Result<()> {
    fn read_and_print_file(file_path: &Path, placeholder: &str) -> Result<()> {
        if !file_path.exists() {
            return Ok(());
        }

        // Check if the file is empty before opening it
        let metadata = std::fs::metadata(file_path)?;
        if metadata.len() == 0 {
            return Ok(());
        }

        let file = File::open(file_path)?;
        let reader = std::io::BufReader::new(file);
        let mut start = true;

        use std::io::BufRead;

        for line in reader.lines() {
            if let Ok(content) = line {
                if start {
                    start = false;
                    println!("{placeholder}");
                }
                println!("{}", content);
            } else if let Err(err) = line {
                eprintln!("Error reading line: {}", err);
            }
        }

        Ok(())
    }

    let stdout_path = Path::new(DEFAULT_STDOUT_PATH);
    read_and_print_file(stdout_path, "STDOUT>")?;

    let stderr_path = Path::new(DEFAULT_STDERR_PATH);
    read_and_print_file(stderr_path, "STDERR>")?;

    Ok(())
}
