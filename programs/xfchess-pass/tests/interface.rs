// The committed IDL and Anchor.toml must name the program ID the source declares.

const IDL: &str = include_str!("../../../idl/xfchess_pass.json");
const ANCHOR_TOML: &str = include_str!("../../../Anchor.toml");

#[test]
fn committed_interface_files_use_the_declared_program_id() {
    let id = xfchess_pass::ID.to_string();
    assert!(IDL.contains(&format!("\"address\": \"{id}\"")));
    assert!(ANCHOR_TOML.contains(&format!("xfchess_pass = \"{id}\"")));
}
