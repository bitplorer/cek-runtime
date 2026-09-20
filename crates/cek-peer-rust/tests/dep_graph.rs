//! Native peer crate must not depend on the WASM hop (no sibling→sibling).

#[test]
fn rust_does_not_depend_on_peer_wasm() {
    let toml = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml"));
    let deps = direct_dep_names(toml);
    assert!(
        !deps.iter().any(|n| *n == "cek-peer-wasm"),
        "cek-peer-rust must not depend on cek-peer-wasm (sibling→sibling)"
    );
    assert!(
        deps.iter().any(|n| *n == "cek-peer-kernel"),
        "cek-peer-rust must depend on cek-peer-kernel only for apply"
    );
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
