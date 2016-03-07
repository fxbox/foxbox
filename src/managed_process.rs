/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

// Assumes Unix
use libc::{self,  c_int};

use std::thread;
use std::thread::JoinHandle;
use std::sync::{ Arc, Mutex };
use std::process::Child;
use std::io::{ Error, ErrorKind, Result };
use std::time::{ SystemTime, Duration, UNIX_EPOCH };

/// Unix exit statuses
#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub struct ExitStatus(c_int);

const RESTART_TIME_THRESHOLD: f64 = 5.0; // seconds

fn seconds_since_epoch() -> f64 {
    let now = SystemTime::now();
    now.duration_since(UNIX_EPOCH).unwrap().as_secs() as f64
}

pub struct ManagedProcess {
    kill_signal: Arc<Mutex<u32>>,
    pid:         Arc<Mutex<Option<u32>>>,
    thread:      JoinHandle<()>
}

impl ManagedProcess {

    /// Create a new ManagedProcess and start it.
    ///
    /// # Examples
    ///
    /// ```
    /// use tunnel_controller::ManagedProcess;
    /// use std::process::Command;
    ///
    /// let process = ManagedProcess::start(|| {
    ///     Command::new("echo")
    ///             .arg("Hello")
    ///             .arg("World")
    ///             .spawn()
    /// });
    ///
    /// ```
    pub fn start<F: 'static>(spawn: F) -> Result<ManagedProcess>
        where F: Fn() -> Result<Child> + Send {

        let pid = Arc::new(Mutex::new(None));

        // Uses a u32 Mutex to avoid the compiler complaining that you can use an AtomicBool.
        // In this case we want a bool like thing _and_ a lock.
        let kill_signal = Arc::new(Mutex::new(0));

        let shared_kill_signal  = kill_signal.clone();
        let shared_pid = pid.clone();

        let thread = thread::spawn(move || {
            // Artificial start/end time for first run
            let mut start_time = RESTART_TIME_THRESHOLD as f64;
            let mut end_time = 0 as f64;
            let mut backoff = 0;
            let mut starts = 0;

            loop {
                let mut child_process;

                {
                    let kill_signal = shared_kill_signal.lock().unwrap();
                    let mut pid = shared_pid.lock().unwrap();

                    if *kill_signal == 1 {
                        *pid = None;
                        debug!("Received process kill signal");
                        break;
                    }

                    if (end_time - start_time) < RESTART_TIME_THRESHOLD {
                       backoff += 1;
                       let backoff_seconds = (backoff * backoff) / 2;
                       info!("Backing off creating a new process for {} seconds", backoff_seconds);
                       thread::sleep(Duration::new(backoff_seconds, 0));
                    } else {
                        backoff = 0;
                    }

                    info!("Starting process. Restarted {} times", starts);
                    start_time = seconds_since_epoch();
                    child_process = spawn().unwrap();
                    *pid = Some(child_process.id());
                }

                starts += 1;

                info!("Started managed process pid: {}", child_process.id());
                child_process.wait().unwrap();
                end_time = seconds_since_epoch();
            }
        });

        Ok(ManagedProcess {
            kill_signal: kill_signal,
            pid:  pid,
            thread: thread
        })
    }

    /// Get the current process ID or None if no process is running
    fn get_pid(&self) -> Option<u32> {
        *self.pid.lock().unwrap()
    }

    /// Shut the ManagedProcess down safely. Equivalent to sending SIGKILL to the
    /// running process if it is currently alive
    ///
    /// # Examples
    ///
    /// ```
    /// use tunnel_controller::ManagedProcess;
    /// use std::process::Command;
    ///
    /// let process = ManagedProcess::start(|| {
    ///     Command::new("sleep")
    ///             .arg("10000")
    ///             .spawn()
    /// });
    ///
    /// process.shutdown().unwrap();
    ///
    /// ```
    pub fn shutdown(self) -> Result<()> {

        {
            let mut kill_signal = self.kill_signal.lock().unwrap();
            *kill_signal = 1;
        }

        // If there is no assigned pid, the process is not running.
        let pid = self.get_pid();

        if pid.is_none() {
            self.join_thread();
            return Ok(());
        }

        let pid = pid.unwrap() as i32;

        // if the process has finished, and therefore had waitpid called,
        // and we kill it, then on unix we might ending up killing a
        // newer process that happens to have a re-used id
        let status = try_wait(pid);

        if status.is_some() {
            // Process is already exited
            self.join_thread();
            return Ok(());
        }

        debug!("Sending SIGKILL to pid: {}", pid);
        unsafe { try!(c_rv(libc::kill(pid, libc::SIGKILL))); }

        self.join_thread();
        Ok(())
    }

    /// Wait for the thread to exit
    fn join_thread(self) -> () {
        self.thread.join().unwrap();
    }
}


/// A non-blocking 'wait' for a given process id.
fn try_wait(id: i32) -> Option<ExitStatus> {
    let mut status = 0 as c_int;

    match c_rv_retry(|| unsafe {
        libc::waitpid(id, &mut status, libc::WNOHANG)
    }) {
        Ok(0)  => None,
        Ok(n) if n == id => Some(ExitStatus(status)),
        Ok(n)  => panic!("Unknown pid: {}", n),
        Err(e) => panic!("Unknown waitpid error: {}", e)
    }
}

/// Check the return value of libc function and turn it into a
/// Result type
fn c_rv(t: c_int) -> Result<c_int> {
    if t == -1 {
        Err(Error::last_os_error())
    } else {
        Ok(t)
    }
}

/// Check the return value of a libc function, but, retry the given function if
/// the returned error is EINTR (Interrupted)
fn c_rv_retry<F>(mut f: F) -> Result<c_int>
    where F: FnMut() -> c_int
{
    loop {
        match c_rv(f()) {
            Err(ref e) if e.kind() == ErrorKind::Interrupted => {}
            other => return other,
        }
    }
}

#[test]
fn test_managed_process_restart() {
    use std::sync::mpsc::channel;
    use std::process::Command;

    let (counter_tx, counter_rx) = channel();

    let process = ManagedProcess::start(move || {
        counter_tx.send(1).unwrap();

        Command::new("sleep")
                .arg("0")
                .spawn()
    }).unwrap();

    let mut count = 0;

    // Maybe spin with try_recv and check a duration
    // to assert liveness?
    while count < 2 {
        count = count + counter_rx.recv().unwrap()
    }

    process.shutdown().unwrap();
}

#[test]
fn test_managed_process_shutdown() {
    use std::process::Command;
    // Ideally need a timeout. The test should be, if shutdown doesn't happen immediately,
    // something's broken.
    let process = ManagedProcess::start(|| {
        Command::new("sleep")
                .arg("1000")
                .spawn()
    }).unwrap();

    process.shutdown().unwrap();
}
