use std::ffi::{CStr, CString};
use std::fmt::{Display, Formatter};
use std::os::raw::c_char;
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard};

#[cfg_attr(
    all(target_os = "windows", target_env = "msvc"),
    link(name = "golassie.dll")
)]
#[cfg_attr(
    not(all(target_os = "windows", target_env = "msvc")),
    link(name = "golassie")
)]
extern "C" {
    fn InitDaemon(config: *const GoDaemonConfig) -> InitDaemonResult;
    fn DropDaemonInitResult(result: *mut InitDaemonResult);
    fn RunDaemon() -> LassieResult;
    fn StopDaemon() -> LassieResult;
    fn DropResult(value: *mut LassieResult);
}

#[repr(C)]
#[derive(Debug)]
struct InitDaemonResult {
    port: u16,
    error: *const c_char,
}

impl Drop for InitDaemonResult {
    fn drop(&mut self) {
        unsafe { DropDaemonInitResult(self) }
    }
}

impl InitDaemonResult {
    fn error(&self) -> Option<String> {
        from_c_string(self.error)
    }
}

#[repr(C)]
#[derive(Debug)]
struct LassieResult {
    error: *const c_char,
}

impl Drop for LassieResult {
    fn drop(&mut self) {
        unsafe { DropResult(self) }
    }
}

impl LassieResult {
    fn error(&self) -> Option<String> {
        from_c_string(self.error)
    }
}

fn from_c_string(str: *const c_char) -> Option<String> {
    if str.is_null() {
        return None;
    }

    Some(unsafe { CStr::from_ptr(str) }.to_string_lossy().to_string())
}

#[repr(C)]
struct GoDaemonConfig {
    // this must be kept in sync with the definition of daemon_config_t in go-lib/lassie-ffi.go
    temp_dir: *const c_char,
    port: u16,
    log_level: usize,
}

struct GoDaemon {
    handler_thread: std::thread::JoinHandle<()>,
}

static mut DAEMON: Mutex<Option<GoDaemon>> = Mutex::new(None);
fn get_global_daemon() -> std::sync::LockResult<MutexGuard<'static, Option<GoDaemon>>> {
    unsafe { DAEMON.lock() }
}

#[derive(Debug, Clone, Default)]
pub struct DaemonConfig {
    temp_dir: Option<PathBuf>,
    port: u16,
}

pub struct Daemon {
    port: u16,
}

impl Daemon {
    pub fn start(config: DaemonConfig) -> Result<Self, StartError> {
        log::debug!("[Daemon::start] Locking global daemon mutex");
        let mut maybe_daemon = get_global_daemon().map_err(|_| StartError::MutexPoisoned)?;
        if maybe_daemon.is_some() {
            log::error!("{}", StartError::OnlyOneInstanceAllowed);
            return Err(StartError::OnlyOneInstanceAllowed);
        }

        log::info!("Starting Lassie Daemon");
        let temp_dir = match config.temp_dir {
            None => "".to_string(),
            Some(dir) => {
                let str = dir.to_str();
                match str {
                    None => return Err(StartError::PathIsNotValidUtf8(dir)),
                    Some(val) => val.to_string(),
                }
            }
        };

        let temp_dir = CString::new(temp_dir.clone())
            .map_err(|_| StartError::PathContainsNullByte(temp_dir))?;

        let log_level = if log::log_enabled!(log::Level::Debug) {
            log::LevelFilter::Debug
        } else {
            log::LevelFilter::Off
        };
        let go_config = GoDaemonConfig {
            temp_dir: temp_dir.as_ptr(),
            log_level: log_level as usize,
            port: config.port,
        };

        let result = unsafe { InitDaemon(&go_config) };
        log::debug!("Lassie.InitDaemon result: {:?}", result);

        if let Some(msg) = result.error() {
            log::error!("Lassie.InitDaemon failed: {msg}");
            return Err(StartError::Lassie(msg));
        }
        let port = result.port;
        log::debug!("Lassie.InitDaemon returned port: {port}");

        let handler_thread = std::thread::spawn(|| {
            log::debug!("Running Lassie HTTP handler");
            let result = unsafe { RunDaemon() };
            if let Some(msg) = result.error() {
                log::error!("Lassie HTTP handler failed: {msg}");
                // TODO: should we somehow notify the main thread about the problem?
                // Maybe we should panic? That would not kill the main thread though.
            }
            log::debug!("HTTP handler exited");
        });
        *maybe_daemon = Some(GoDaemon { handler_thread });

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
        let result = unsafe { StopDaemon() };
        if let Some(msg) = result.error() {
            panic!("Cannot stop Lassie Daemon: {msg}");
        }

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
    PathContainsNullByte(String),
    PathIsNotValidUtf8(PathBuf),
    Lassie(String),
}

impl Display for StartError {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "failed to start Lassie daemon: ")?;
        match self {
            StartError::MutexPoisoned => f.write_str("the global mutex was poisoned"),
            StartError::OnlyOneInstanceAllowed => {
                f.write_str("cannot create more than one instance")
            }
            StartError::PathContainsNullByte(path_str) => f.write_fmt(format_args!(
                "null bytes are not allowed in paths (value: {:?})",
                path_str
            )),
            StartError::PathIsNotValidUtf8(path) => f.write_fmt(format_args!(
                "path that are not valid UTF-8 are not supported (value: {:?})",
                path.display(),
            )),
            StartError::Lassie(msg) => f.write_str(msg),
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

        let daemon = Daemon::start(DaemonConfig::default()).expect("cannot start Lassie");
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
        let d = Daemon::start(DaemonConfig::default()).expect("cannot start the first time");
        drop(d);
        let _ = Daemon::start(DaemonConfig::default()).expect("cannot start the second time");
    }

    #[test]
    fn cannot_start_twice() {
        let _lock = setup_test_env();
        let _first =
            Daemon::start(DaemonConfig::default()).expect("cannot start the first instance");
        match Daemon::start(DaemonConfig::default()) {
            Ok(_) => panic!("starting another instance should have failed"),

            Err(err) => assert_eq!(err, StartError::OnlyOneInstanceAllowed),
        };
    }

    #[test]
    #[cfg_attr(windows, ignore)]
    fn reports_listen_error() {
        let _lock = setup_test_env();
        let result = Daemon::start(DaemonConfig {
            port: 1,
            ..DaemonConfig::default()
        });
        match result {
            Ok(_) => panic!("starting Lassie on port 1 should have failed"),
            Err(StartError::Lassie(msg)) => {
                assert!(
                    msg.contains("cannot start the HTTP server")
                        && msg.contains("listen tcp 127.0.0.1:1")
                        && msg.contains("permission denied"),
                    "Expected bind-socket permission error, actual: {msg}",
                );
            }
            Err(err) => panic!("unexpected error while starting Lassie on port 1: {err}"),
        };
    }

    fn setup_test_env() -> MutexGuard<'static, ()> {
        let _ = env_logger::builder().is_test(true).try_init();
        let lock = TEST_GUARD.lock().expect("cannot obtain global test lock. This typically happens when one of the test fails; the problem should go away after you fix the test failure.");
        lock
    }
}
