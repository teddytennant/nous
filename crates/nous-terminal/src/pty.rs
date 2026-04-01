//! PTY (pseudo-terminal) management for Unix systems.
//!
//! Spawns a child process in a PTY, provides read/write access
//! to the master side, and handles resizing via SIGWINCH.

use crate::{Result, TerminalError};
use nix::pty::{OpenptyResult, openpty};
use nix::sys::wait::{WaitPidFlag, waitpid};
use nix::unistd::{ForkResult, Pid, execvp, fork, setsid};
use std::ffi::CString;
use std::os::fd::{AsRawFd, OwnedFd, RawFd};

/// PTY dimensions.
#[derive(Debug, Clone, Copy)]
pub struct PtySize {
    pub rows: u16,
    pub cols: u16,
}

/// A spawned PTY with a child process.
pub struct Pty {
    master: OwnedFd,
    child_pid: Pid,
}

impl Pty {
    /// Spawn a shell in a new PTY.
    pub fn spawn(shell: &str, size: &PtySize) -> Result<Self> {
        let winsize = libc::winsize {
            ws_row: size.rows,
            ws_col: size.cols,
            ws_xpixel: 0,
            ws_ypixel: 0,
        };

        let OpenptyResult { master, slave } =
            openpty(Some(&winsize), None).map_err(|e| TerminalError::Pty(e.to_string()))?;

        match unsafe { fork() }.map_err(|e| TerminalError::Pty(e.to_string()))? {
            ForkResult::Parent { child } => {
                drop(slave);
                set_nonblocking(master.as_raw_fd())?;
                Ok(Self {
                    master,
                    child_pid: child,
                })
            }
            ForkResult::Child => {
                drop(master);

                setsid().ok();

                let slave_fd = slave.as_raw_fd();

                // Make slave the controlling terminal
                unsafe {
                    libc::ioctl(slave_fd, libc::TIOCSCTTY as libc::c_ulong, 0);
                }

                // Redirect stdin/stdout/stderr to slave via raw libc
                unsafe {
                    libc::dup2(slave_fd, 0);
                    libc::dup2(slave_fd, 1);
                    libc::dup2(slave_fd, 2);
                }

                if slave_fd > 2 {
                    drop(slave);
                }

                // Set TERM
                unsafe {
                    std::env::set_var("TERM", "xterm-256color");
                }

                let shell_cstr =
                    CString::new(shell).map_err(|_| TerminalError::Pty("invalid shell".into()))?;
                execvp(&shell_cstr, &[&shell_cstr])
                    .map_err(|e| TerminalError::Pty(format!("exec failed: {e}")))?;

                unreachable!()
            }
        }
    }

    /// Write data to the master side of the PTY.
    pub fn write_all(&mut self, data: &[u8]) -> Result<()> {
        let fd = self.master.as_raw_fd();
        let mut offset = 0;
        while offset < data.len() {
            let n = unsafe {
                libc::write(
                    fd,
                    data[offset..].as_ptr() as *const libc::c_void,
                    data.len() - offset,
                )
            };
            if n < 0 {
                let err = std::io::Error::last_os_error();
                if err.kind() == std::io::ErrorKind::Interrupted {
                    continue;
                }
                return Err(TerminalError::Pty(format!("write failed: {err}")));
            }
            offset += n as usize;
        }
        Ok(())
    }

    /// Try to read available data from the PTY. Non-blocking.
    pub fn try_read(&mut self) -> Result<Vec<u8>> {
        let mut buf = vec![0u8; 8192];
        let n = unsafe {
            libc::read(
                self.master.as_raw_fd(),
                buf.as_mut_ptr() as *mut libc::c_void,
                buf.len(),
            )
        };
        if n > 0 {
            buf.truncate(n as usize);
            Ok(buf)
        } else if n == 0 {
            Ok(vec![])
        } else {
            let err = std::io::Error::last_os_error();
            match err.raw_os_error() {
                Some(libc::EAGAIN) => Ok(vec![]),
                Some(libc::EIO) => Ok(vec![]), // child exited
                _ => Err(TerminalError::Pty(format!("read failed: {err}"))),
            }
        }
    }

    /// Resize the PTY.
    pub fn resize(&mut self, size: &PtySize) -> Result<()> {
        let winsize = libc::winsize {
            ws_row: size.rows,
            ws_col: size.cols,
            ws_xpixel: 0,
            ws_ypixel: 0,
        };

        let ret = unsafe { libc::ioctl(self.master.as_raw_fd(), libc::TIOCSWINSZ, &winsize) };
        if ret < 0 {
            return Err(TerminalError::Pty("resize ioctl failed".into()));
        }

        nix::sys::signal::kill(self.child_pid, nix::sys::signal::Signal::SIGWINCH)
            .map_err(|e| TerminalError::Pty(format!("sigwinch failed: {e}")))?;

        Ok(())
    }

    /// Check if the child process is still alive.
    pub fn is_alive(&self) -> bool {
        matches!(
            waitpid(self.child_pid, Some(WaitPidFlag::WNOHANG)),
            Ok(nix::sys::wait::WaitStatus::StillAlive)
        )
    }

    /// Get the master file descriptor for polling.
    pub fn master_fd(&self) -> RawFd {
        self.master.as_raw_fd()
    }
}

impl Drop for Pty {
    fn drop(&mut self) {
        let _ = nix::sys::signal::kill(self.child_pid, nix::sys::signal::Signal::SIGHUP);
    }
}

/// Set a file descriptor to non-blocking mode.
fn set_nonblocking(fd: RawFd) -> Result<()> {
    let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
    if flags < 0 {
        return Err(TerminalError::Pty("fcntl F_GETFL failed".into()));
    }
    let ret = unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) };
    if ret < 0 {
        return Err(TerminalError::Pty("fcntl F_SETFL failed".into()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spawn_shell() {
        let size = PtySize { rows: 24, cols: 80 };
        let pty = Pty::spawn("/bin/sh", &size).unwrap();
        assert!(pty.is_alive());
    }

    #[test]
    fn write_and_read() {
        let size = PtySize { rows: 24, cols: 80 };
        let mut pty = Pty::spawn("/bin/sh", &size).unwrap();

        pty.write_all(b"echo nous_test_output\n").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(100));

        let output = pty.try_read().unwrap();
        assert!(!output.is_empty());
    }

    #[test]
    fn resize_pty() {
        let size = PtySize { rows: 24, cols: 80 };
        let mut pty = Pty::spawn("/bin/sh", &size).unwrap();

        let new_size = PtySize {
            rows: 40,
            cols: 120,
        };
        pty.resize(&new_size).unwrap();
        assert!(pty.is_alive());
    }

    #[test]
    fn nonblocking_read_empty() {
        let size = PtySize { rows: 24, cols: 80 };
        let mut pty = Pty::spawn("/bin/sh", &size).unwrap();

        std::thread::sleep(std::time::Duration::from_millis(50));
        let _ = pty.try_read();

        let data = pty.try_read().unwrap();
        assert!(data.len() < 8192);
    }

    #[test]
    fn child_exit_detection() {
        let size = PtySize { rows: 24, cols: 80 };
        let mut pty = Pty::spawn("/bin/sh", &size).unwrap();
        assert!(pty.is_alive());

        pty.write_all(b"exit\n").unwrap();

        // Poll for exit with retries (process may take time on loaded systems)
        let mut exited = false;
        for _ in 0..20 {
            std::thread::sleep(std::time::Duration::from_millis(50));
            if !pty.is_alive() {
                exited = true;
                break;
            }
        }
        assert!(exited, "child process did not exit within 1 second");
    }
}
