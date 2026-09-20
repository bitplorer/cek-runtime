//! Twin JSON wire: rust + wasm `ApplyRequest` / `ApplyResponse` field names
//! must stay equal. Hops own serde; this lock is not a shared JSON crate.

#[test]
fn apply_request_response_fields_match_wasm_twin() {
    let rust = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/lib.rs"));
    let wasm = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../cek-peer-wasm/src/lib.rs"
    ));
    let rust_req = pub_fields(rust, "ApplyRequest");
    let wasm_req = pub_fields(wasm, "ApplyRequest");
    let rust_resp = pub_fields(rust, "ApplyResponse");
    let wasm_resp = pub_fields(wasm, "ApplyResponse");
    assert_eq!(
        rust_req,
        vec!["result", "profile", "unknown_op_policy"],
        "native ApplyRequest public freeze"
    );
    assert_eq!(
        rust_resp,
        vec!["receipt", "kv", "ui", "log"],
        "native ApplyResponse public freeze"
    );
    assert_eq!(rust_req, wasm_req, "ApplyRequest fields drifted across hops");
    assert_eq!(
        rust_resp, wasm_resp,
        "ApplyResponse fields drifted across hops"
    );
}

fn pub_fields(src: &str, struct_name: &str) -> Vec<String> {
    let needle = format!("pub struct {struct_name}");
    let start = src
        .find(&needle)
        .unwrap_or_else(|| panic!("missing {struct_name}"));
    let rest = &src[start..];
    let open = rest.find('{').expect("struct body");
    let mut depth = 0usize;
    let mut close = open;
    for (i, c) in rest[open..].char_indices() {
        match c {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    close = open + i;
                    break;
                }
            }
            _ => {}
        }
    }
    rest[open + 1..close]
        .lines()
        .filter_map(|line| {
            let t = line.trim().strip_prefix("pub ")?;
            let name = t.split(':').next()?.trim();
            if name.is_empty() || name.contains('(') {
                return None;
            }
            Some(name.to_string())
        })
        .collect()
}
