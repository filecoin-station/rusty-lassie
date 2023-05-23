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
    use pretty_assertions::assert_eq;

    #[test]
    fn start_server() {
        let port = Daemon::start();
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
        // FIXME
        // assert_eq!(
        //     response.header("Content-Type"),
        //     Some("application/vnd.ipld.car")
        // );

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
    fn shutdown_server() {
        Daemon::start();
        Daemon::stop();
    }
}
