//! A listening endpoint catches any attempt to upload, ping, fetch counts or delete remotely.
//! Old consent and queued reports must not bypass this fork's permanent removal.
use horizon_telemetry::payload::{Bug, Crash, Perf};
use horizon_telemetry::upload::{self, Flush};
use horizon_telemetry::{Choices, Consent, Item, Payload, Source, live, outbox};
use std::net::TcpListener;

#[test]
fn old_consent_and_explicit_urls_cannot_send_or_collect() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let home = std::env::temp_dir().join(format!(
        "horizon-clean-{}-{}",
        std::process::id(),
        live::session_id()
    ));
    std::fs::create_dir_all(&home).unwrap();
    let mut consent = Consent::default();
    consent.answer(Choices::ALL);
    consent.save(&home).unwrap();
    let consent_before = std::fs::read(Consent::path(&home)).unwrap();
    for payload in [
        Payload::Crash(Crash::default()),
        Payload::Perf(Perf::default()),
        Payload::Bug(Bug {
            text: "a report".into(),
            ..Bug::default()
        }),
        Payload::Benchmark(serde_json::json!({"benchmark": true})),
    ] {
        let item = Item::new(Source::Game, "clean", payload);
        assert!(!outbox::push(&home, &item).unwrap());
    }
    assert!(!outbox::dir(&home).exists());
    // A report left by an older unmodified game must not leave either.
    std::fs::create_dir_all(outbox::dir(&home)).unwrap();
    let old = outbox::dir(&home).join("old.json");
    std::fs::write(
        &old,
        serde_json::to_vec(&Item::new(
            Source::Game,
            "old",
            Payload::Crash(Crash::default()),
        ))
        .unwrap(),
    )
    .unwrap();
    assert_eq!(upload::flush(&home, &url), Flush::Nothing);
    assert_eq!(upload::flush(&home, &url), Flush::Nothing);
    assert_eq!(upload::preview(&home), "[]");
    assert!(upload::delete_remote(&url, &consent.install_id).is_err());
    assert!(live::send(&url, &live::Ping::new(&live::session_id(), "clean", false)).is_err());
    assert!(live::fetch(&url).is_err());
    assert!(horizon_telemetry::endpoint().is_none());
    assert!(matches!(listener.accept(), Err(e) if e.kind() == std::io::ErrorKind::WouldBlock));
    assert_eq!(std::fs::read(Consent::path(&home)).unwrap(), consent_before);
    assert!(!horizon_telemetry::dir(&home).join("upload.json").exists());
    assert!(old.exists(), "do not delete existing user data silently");
    std::fs::remove_dir_all(home).unwrap();
}
