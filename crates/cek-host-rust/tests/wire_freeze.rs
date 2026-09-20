//! Public Host wire verbs stay the existing host-json surface: mint | submit.

#[test]
fn host_json_cmds_are_mint_and_submit_only() {
    let src = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/lib.rs"));
    let prod = src
        .split("#[cfg(test)]")
        .next()
        .expect("production source");
    assert!(
        prod.contains("\"mint\""),
        "mint wire verb missing from host-json door"
    );
    assert!(
        prod.contains("\"submit\""),
        "submit wire verb missing from host-json door"
    );
    assert!(
        !prod.contains("\"receipt\""),
        "do not invent a receipt wire verb; kernel report_receipt stays off this door"
    );
    assert!(
        !prod.contains("\"reverse\""),
        "do not invent a reverse wire verb; kernel end_activity stays off this door"
    );
    assert!(
        prod.contains("cmd must be mint|submit"),
        "unknown-cmd error literal drifted"
    );
}
