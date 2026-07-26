use std::path::PathBuf;

use tauri::utils::config::Config;

#[test]
fn asset_protocol_only_exposes_the_generated_preview_tree() {
    let config: Config = serde_json::from_str(include_str!("../tauri.conf.json")).unwrap();
    let asset_protocol = config.app.security.asset_protocol;

    assert!(asset_protocol.enable);
    assert_eq!(
        asset_protocol.scope.allowed_paths(),
        &[PathBuf::from("$APPLOCALDATA/assets/previews/**/*")]
    );
}
