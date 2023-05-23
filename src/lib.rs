use std::fmt::{Display, Formatter};
use std::sync::{Mutex, MutexGuard};

#[link(name = "golassie")]
extern "C" {
    fn InitDaemon(debug_log: bool) -> u16;
    fn RunDaemon();
    fn StopDaemon();
}

struct GoDaemon {
    handler_thread: std::thread::JoinHandle<()>,
}

static mut DAEMON: Mutex<Option<GoDaemon>> = Mutex::new(None);
fn get_global_daemon() -> std::sync::LockResult<MutexGuard<'static, Option<GoDaemon>>> {
    unsafe { DAEMON.lock() }
}

pub struct Daemon {
    port: u16,
}

impl Daemon {
    pub fn start() -> Result<Self, StartError> {
        log::debug!("[Daemon::start] Locking global daemon mutex");
        let mut maybe_daemon = get_global_daemon().map_err(|_| StartError::MutexPoisoned)?;
        if maybe_daemon.is_some() {
            log::error!("{}", StartError::OnlyOneInstanceAllowed);
            return Err(StartError::OnlyOneInstanceAllowed);
        }

        log::info!("Starting Lassie Daemon");
        let debug_log_enabled = log::log_enabled!(log::Level::Debug);
        let port = unsafe { InitDaemon(debug_log_enabled) };

        let handler_thread = std::thread::spawn(move || {
            log::debug!("Running Lassie HTTP handler");
            unsafe { RunDaemon() };
            log::debug!("HTTP handler exited");
        });
        maybe_daemon.replace(GoDaemon { handler_thread });

        log::info!("Lassie Daemon is listening on port {}", port);
        Ok(Daemon { port })
    }

    pub fn port(&self) -> u16 {
        self.port
    }
}

impl Drop for Daemon {
    fn drop(&mut self) {
        log::debug!("[Daemon::drop] Locking global daemon mutex");
        let mut maybe_daemon = get_global_daemon().expect("global daemon mutex was poisoned");
        if maybe_daemon.is_none() {
            panic!("Daemon.drop() was called when no GoDaemon was running");
        }

        log::debug!("Shutting down Lassie Daemon");
        unsafe { StopDaemon() };

        log::debug!("Waiting for Lassie to exit");
        // It's safe to call unwrap() here because we already handled maybe_daemon.is_none() above
        let GoDaemon { handler_thread } = maybe_daemon.take().unwrap();
        handler_thread.join().expect("Lassie handler panicked");
    }
}

#[derive(Debug, PartialEq, Clone)]
#[non_exhaustive]
pub enum StartError {
    MutexPoisoned,
    OnlyOneInstanceAllowed,
    // todo: Lassie errors
}

impl Display for StartError {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "failed to start Lassie daemon: ")?;
        match self {
            StartError::MutexPoisoned => f.write_str("the global mutex was poisoned"),
            StartError::OnlyOneInstanceAllowed => {
                f.write_str("cannot create more than one instance")
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use pretty_assertions::assert_eq;

    // Rust runs tests in parallel. Since Lassie Daemon is a singleton,
    // we must synchronise the tests to ensure they run sequentially
    static TEST_GUARD: Mutex<()> = Mutex::new(());

    #[test]
    fn start_daemon_and_request_cid() {
        let _lock = setup_test_env();

        let daemon = Daemon::start().expect("cannot start Lassie");
        let port = daemon.port();
        assert!(port > 0, "Lassie is listening on non-zero port number");

        let url = format!("http://127.0.0.1:{port}/ipfs/bafybeib36krhffuh3cupjml4re2wfxldredkir5wti3dttulyemre7xkni");
        let response = ureq::get(&url)
            .set("Accept", "application/vnd.ipld.car")
            .call();

        if let Err(ureq::Error::Status(code, response)) = response {
            panic!(
                "Request failed with status {}. Body:\n{}\n==EOF==",
                code,
                response.into_string().expect("cannot read response body")
            );
        }

        let response = response.expect("cannot fetch CID bzfybeib...7xkni");

        println!("response headers: {:?}", response.headers_names());
        for hn in &response.headers_names() {
            println!("\t{hn}: {}", response.header(hn).unwrap_or("<empty>"));
        }

        if response.status() != 200 {}

        assert_eq!(response.status(), 200);
        assert_eq!(
            response.header("Content-Type"),
            Some("application/vnd.ipld.car; version=1")
        );

        let mut content = Vec::new();
        response
            .into_reader()
            .read_to_end(&mut content)
            .expect("cannot read response body");

        assert_eq!(
            content,
            include_bytes!(
                "../testdata/bafybeib36krhffuh3cupjml4re2wfxldredkir5wti3dttulyemre7xkni.car"
            )
        );
    }

    #[test]
    fn can_start_after_stopping() {
        let _lock = setup_test_env();
        let d = Daemon::start().expect("cannot start the first time");
        drop(d);
        let _ = Daemon::start().expect("cannot start the second time");
    }

    #[test]
    fn cannot_start_twice() {
        let _lock = setup_test_env();
        let _first = Daemon::start().expect("cannot start the first instance");
        match Daemon::start() {
            Ok(_) => panic!("starting another instance should have failed"),
            Err(err) => assert_eq!(err, StartError::OnlyOneInstanceAllowed),
        };
    }

    fn setup_test_env() -> MutexGuard<'static, ()> {
        let _ = env_logger::builder().is_test(true).try_init();
        let lock = TEST_GUARD.lock().expect("cannot obtain global test lock");
        lock
    }
}
