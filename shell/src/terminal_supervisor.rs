//! One lightweight subreaper per terminal. Detached descendants remain owned
//! by this supervisor and are stopped when the shell exits or Terminal quits.
use std::os::unix::process::CommandExt;
use std::sync::atomic::{AtomicBool, Ordering};
static STOP: AtomicBool = AtomicBool::new(false);
extern "C" fn stop(_: i32) { STOP.store(true, Ordering::Relaxed); }

pub fn run() -> i32 {
    let mut args = std::env::args_os().skip(2);
    let Some(program) = args.next() else { return 127 };
    // This executable enters here before GTK, threads or application startup.
    unsafe {
        if libc::prctl(libc::PR_SET_CHILD_SUBREAPER, 1, 0, 0, 0) < 0 { return 127; }
        let mut action: libc::sigaction = std::mem::zeroed();
        action.sa_sigaction = stop as *const () as usize;
        libc::sigemptyset(&mut action.sa_mask);
        for signal in [libc::SIGTERM, libc::SIGHUP, libc::SIGINT] { libc::sigaction(signal, &action, std::ptr::null_mut()); }
        libc::signal(libc::SIGTTOU, libc::SIG_IGN);
        libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGTERM);
        if libc::getppid() == 1 { return 127; }
    }
    // Sleep on child/termination signals instead of polling each idle pane.
    let mut waiting: libc::sigset_t = unsafe { std::mem::zeroed() };
    unsafe {
        libc::sigemptyset(&mut waiting);
        for signal in [libc::SIGCHLD, libc::SIGTERM, libc::SIGHUP, libc::SIGINT] { libc::sigaddset(&mut waiting, signal); }
        libc::sigprocmask(libc::SIG_BLOCK, &waiting, std::ptr::null_mut());
    }
    let mut command = std::process::Command::new(program);
    command.args(args);
    unsafe {
        command.pre_exec(|| {
            let mut mask: libc::sigset_t = std::mem::zeroed();
            libc::sigemptyset(&mut mask); libc::sigprocmask(libc::SIG_SETMASK, &mask, std::ptr::null_mut());
            if libc::setpgid(0, 0) < 0 || libc::tcsetpgrp(0, libc::getpid()) < 0 { return Err(std::io::Error::last_os_error()); }
            for signal in [libc::SIGINT, libc::SIGQUIT, libc::SIGTERM, libc::SIGHUP, libc::SIGTTOU, libc::SIGTTIN] { libc::signal(signal, libc::SIG_DFL); }
            Ok(())
        });
    }
    let child = match command.spawn() { Ok(child) => child, Err(error) => { eprintln!("Terminal: {error}"); return 127; } };
    let main_pid = child.id() as i32;
    let mut code = 0;
    while !STOP.load(Ordering::Relaxed) {
        let mut status = 0;
        let pid = unsafe { libc::waitpid(-1, &mut status, libc::WNOHANG) };
        if pid == main_pid {
            code = if libc::WIFEXITED(status) { libc::WEXITSTATUS(status) } else { 128 + libc::WTERMSIG(status) };
            break;
        }
        if pid == 0 {
            let signal = unsafe { libc::sigwaitinfo(&waiting, std::ptr::null_mut()) };
            if signal > 0 && signal != libc::SIGCHLD { STOP.store(true, Ordering::Relaxed); }
        }
        if pid < 0 && std::io::Error::last_os_error().raw_os_error() != Some(libc::EINTR) { break; }
    }
    // Repeat discovery as children are reparented to us; no daemonized child
    // can escape by changing its process group or session ID.
    let start = std::time::Instant::now();
    loop {
        let descendants = descendants(std::process::id() as i32);
        for pid in descendants {
            signal_pid(pid, if start.elapsed().as_millis() < 400 { libc::SIGTERM } else { libc::SIGKILL });
        }
        loop {
            let result = unsafe { libc::waitpid(-1, std::ptr::null_mut(), libc::WNOHANG) };
            if result == 0 { break; }
            if result < 0 {
                if std::io::Error::last_os_error().raw_os_error() == Some(libc::ECHILD) { return code; }
                break;
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(if start.elapsed().as_secs() > 1 { 250 } else { 25 }));
    }
}
pub(crate) fn descendants(root: i32) -> Vec<i32> {
    let mut pairs = Vec::new();
    if let Ok(entries) = std::fs::read_dir("/proc") {
        for entry in entries.flatten() {
            let Ok(pid) = entry.file_name().to_string_lossy().parse::<i32>() else { continue };
            let Ok(stat) = std::fs::read_to_string(entry.path().join("stat")) else { continue };
            let Some((_, tail)) = stat.rsplit_once(')') else { continue };
            if let Some(parent) = tail.split_whitespace().nth(1).and_then(|v| v.parse::<i32>().ok()) { pairs.push((pid, parent)); }
        }
    }
    let mut found = vec![root];
    let mut i = 0;
    while i < found.len() {
        let parent = found[i];
        for &(pid, ppid) in &pairs { if ppid == parent && !found.contains(&pid) { found.push(pid); } }
        i += 1;
    }
    found.remove(0); found
}
fn signal_pid(pid: i32, signal: i32) {
    // Open a stable process reference, then recheck ownership before signaling.
    let fd = unsafe { libc::syscall(libc::SYS_pidfd_open, pid, 0) as i32 };
    if fd < 0 { return; }
    if descendants(std::process::id() as i32).contains(&pid) {
        unsafe { libc::syscall(libc::SYS_pidfd_send_signal, fd, signal, std::ptr::null::<libc::siginfo_t>(), 0); }
    }
    unsafe { libc::close(fd); }
}
