//! #10 put a wasm → rust sibling edge (wrong owner). That edge is gone.

#[test]
fn wasm_does_not_depend_on_peer_rust() {
    let toml = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml"));
    let deps = direct_dep_names(toml);
    assert!(
        !deps.iter().any(|n| *n == "cek-peer-rust"),
        "cek-peer-wasm must not depend on cek-peer-rust (sibling→sibling; #10 wrong-owner)"
    );
    assert!(
        deps.iter().any(|n| *n == "cek-peer-kernel"),
        "cek-peer-wasm must depend on cek-peer-kernel only for apply"
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
