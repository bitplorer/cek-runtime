//! `cek host-json` hops onto this crate; it does not talk to the host kernel directly.

#[test]
fn cli_host_json_hops_onto_host_runtime() {
    let main = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../cek-cli/src/main.rs"
    ));
    let start = main
        .find("fn run_host_json")
        .expect("run_host_json missing");
    let body = &main[start..];
    let end = body[3..].find("\nfn ").map(|i| i + 3).unwrap_or(body.len());
    let body = &body[..end];
    assert!(
        body.contains("cek_host_rust::"),
        "cek host-json must hop onto cek-host-rust"
    );
    assert!(
        !body.contains("Host::"),
        "cek host-json must not construct the host kernel Host itself"
    );
}

#[test]
fn cli_manifest_depends_on_host_rust() {
    let toml = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../cek-cli/Cargo.toml"
    ));
    assert!(
        toml.contains("cek-host-rust"),
        "cek-cli must depend on cek-host-rust"
    );
}
