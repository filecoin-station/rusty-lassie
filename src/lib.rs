#[link(name = "golassie")]
extern "C" {
    fn StartDaemon() -> u16;
    fn StopDaemon();
}

pub struct Daemon {}

impl Daemon {
    pub fn start() -> u16 {
        unsafe { StartDaemon() }
    }

    pub fn stop() {
        unsafe { StopDaemon() }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn start_server() {
        let port = Daemon::start();
        assert!(port > 0, "Lassie is listening on non-zero port number");
    }

    #[test]
    fn shutdown_server() {
        Daemon::start();
        Daemon::stop();
    }
}
