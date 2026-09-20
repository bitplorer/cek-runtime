//! Host runtime depends on the Host kernel only. No peer-side coupling.

#[test]
fn rust_depends_on_host_kernel_not_peer_crates() {
    let toml = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml"));
    let deps = direct_dep_names(toml);
    assert!(
        deps.contains(&"cek-host-kernel"),
        "cek-host-rust must depend on cek-host-kernel"
    );
    for forbidden in [
        "cek-peer-kernel",
        "cek-peer-rust",
        "cek-peer-wasm",
        "cek-peer-pyo3",
    ] {
        assert!(
            !deps.contains(&forbidden),
            "cek-host-rust must not depend on {forbidden}"
        );
    }
}

fn direct_dep_names(toml: &str) -> Vec<&str> {
    let mut names = Vec::new();
    let mut in_deps = false;
    for line in toml.lines() {
        let t = line.trim();
        if t.starts_with('[') {
            in_deps = t == "[dependencies]" || t == "[dev-dependencies]";
            continue;
        }
        if in_deps {
            if let Some((name, _)) = t.split_once('=') {
                let name = name.trim();
                if !name.is_empty() && !name.starts_with('#') {
                    names.push(name);
                }
            }
        }
    }
    names
}
