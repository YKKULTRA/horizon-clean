//! Privacy regressions across real CLI dispatch and game-command construction.
use horizon_launcher::{
    backend,
    launch::{self, Launch},
};
use std::process::Command;

#[test]
fn collection_cannot_be_reenabled_through_cli_or_mcp_dispatch() {
    let root = std::env::temp_dir().join(format!("horizon-clean-cli-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_horizon-launcher"))
        .env("HORIZON_LAUNCHER_HOME", root.join("launcher"))
        .env("HORIZON_HOME", root.join("game"))
        .args(["--json", "data", "status"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["enabled"], false);
    assert_eq!(value["removed"], true);
    assert!(value["server"].is_null());
    let output = Command::new(env!("CARGO_BIN_EXE_horizon-launcher"))
        .env("HORIZON_LAUNCHER_HOME", root.join("launcher"))
        .env("HORIZON_HOME", root.join("game"))
        .args(["data", "allow", "all"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("removed"));
    assert!(!root.join("game/telemetry").exists());
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn profile_environment_cannot_restore_legacy_game_pings() {
    for customized in [false, true] {
        let mut profile = Launch::default();
        if customized {
            profile.env.insert(
                "HORIZON_TELEMETRY_URL".into(),
                "https://example.invalid".into(),
            );
            profile.env.insert("HORIZON_NO_HEARTBEAT".into(), "".into());
        }
        let command = launch::wrap(Command::new("game"), &profile);
        let vars: std::collections::BTreeMap<_, _> = command.get_envs().collect();
        assert_eq!(
            vars[std::ffi::OsStr::new("HORIZON_TELEMETRY_URL")],
            Some(std::ffi::OsStr::new("off"))
        );
        assert_eq!(
            vars[std::ffi::OsStr::new("HORIZON_NO_HEARTBEAT")],
            Some(std::ffi::OsStr::new("1"))
        );
    }
}

#[test]
fn upstream_updates_and_server_promotion_stay_disabled() {
    assert!(!backend::self_update_allowed());
    let release = backend::Release {
        tag: "launcher@99.0.0".into(),
        name: "test".into(),
        body: String::new(),
        date: String::new(),
        asset: Some(("http://127.0.0.1:9/should-not-fetch".into(), 0)),
        prerelease: false,
        nightly: None,
    };
    let error = backend::self_update(&release, |_| panic!("no update may start")).unwrap_err();
    assert!(error.contains("disabled"));
    assert!(!backend::Settings::parse(r#"{"install_new_versions":true}"#).auto_install());
    let links = backend::links();
    assert!(links.sponsor.is_empty());
    assert!(links.official_server.is_empty());
}
