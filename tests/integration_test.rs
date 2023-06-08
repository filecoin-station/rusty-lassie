use lassie::{Daemon, DaemonConfig};

#[test]
fn start_daemon_and_request_cid() {
    let _ = env_logger::builder().is_test(true).try_init();

    let daemon = Daemon::start(DaemonConfig::default()).expect("cannot start Lassie");
    let port = daemon.port();
    assert!(port > 0, "Lassie is listening on non-zero port number");

    let url = format!(
        "http://127.0.0.1:{port}/ipfs/bafybeib36krhffuh3cupjml4re2wfxldredkir5wti3dttulyemre7xkni"
    );
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
        include_bytes!("testdata/bafybeib36krhffuh3cupjml4re2wfxldredkir5wti3dttulyemre7xkni.car")
    );
}
