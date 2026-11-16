//! Customer scenarios end to end, against the fake Polar in `common/`.
//! Each test is one story; the names say what the customer does.

mod common;

use common::{run, temp_dir, FakePolar, Status, BENEFIT, OTHER_BENEFIT};
use tauri_plugin_polar_license::{Config, Environment, ErrorKind, LaunchState, Licensing};

const MAC: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const LAPTOP: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

#[test]
fn buys_and_activates() {
    let polar = FakePolar::start();
    polar.issue("KEY-ONE-AAAAAA", BENEFIT, 1);
    let mac = polar.computer(&temp_dir("buys"), MAC, true);

    assert!(!mac.status().licensed, "fresh install shows the gate");
    let status = run(mac.activate("  KEY-ONE-AAAAAA \n")).expect("activates (whitespace trimmed)");
    assert!(status.licensed);
    assert_eq!(status.display_key.as_deref(), Some("****-AAAAAA"));
    assert_eq!(status.environment, "sandbox");
    assert_eq!(polar.activation_count("KEY-ONE-AAAAAA"), 1);
    assert!(
        mac.status().licensed,
        "next launch: licensed from the file alone"
    );
}

#[test]
fn launch_check_confirms_offline_and_ignores_stray_404s() {
    let polar = FakePolar::start();
    polar.issue("KEY-CHECK-AAAAAA", BENEFIT, 1);
    let dir = temp_dir("check");
    let mac = polar.computer(&dir, MAC, false);
    run(mac.activate("KEY-CHECK-AAAAAA")).unwrap();

    assert_eq!(run(mac.launch_check()).state, LaunchState::Confirmed);

    // Offline: nothing listening.
    let mut offline = Config::new(Environment::custom(
        "sandbox",
        "http://127.0.0.1:9",
        common::ORG,
        BENEFIT,
    ));
    offline.log_to_stderr = false;
    let offline = Licensing::new(offline, &dir).with_machine_fingerprint(MAC);
    assert_eq!(run(offline.launch_check()).state, LaunchState::Offline);
    assert!(offline.status().licensed, "offline never removes a license");

    // A 404 that isn't from Polar (proxy / captive portal).
    let mut stray = Config::new(Environment::custom(
        "sandbox",
        format!("{}/not-polar", polar.base),
        common::ORG,
        BENEFIT,
    ));
    stray.log_to_stderr = false;
    let stray = Licensing::new(stray, &dir).with_machine_fingerprint(MAC);
    assert_eq!(run(stray.launch_check()).state, LaunchState::Offline);
    assert!(
        stray.status().licensed,
        "a stray 404 never removes a license"
    );
}

#[test]
fn typo_and_empty_key() {
    let polar = FakePolar::start();
    let mac = polar.computer(&temp_dir("typo"), MAC, true);
    let e = run(mac.activate("NOPE")).unwrap_err();
    assert_eq!(e.kind, ErrorKind::NotRecognized);
    assert!(e.message.starts_with("That license key wasn't recognized."));
    assert!(e
        .message
        .ends_with("Need help? Contact support@example.com."));
    assert_eq!(
        run(mac.activate("   ")).unwrap_err().kind,
        ErrorKind::EmptyKey
    );
}

#[test]
fn key_for_another_product_is_refused_and_slot_returned() {
    let polar = FakePolar::start();
    polar.issue("KEY-OTHER-APP111", OTHER_BENEFIT, 1);
    let mac = polar.computer(&temp_dir("other"), MAC, true);
    let e = run(mac.activate("KEY-OTHER-APP111")).unwrap_err();
    assert_eq!(e.kind, ErrorKind::WrongProduct);
    assert_eq!(
        polar.activation_count("KEY-OTHER-APP111"),
        0,
        "slot handed back"
    );
    assert!(!mac.status().licensed);
}

#[test]
fn not_configured_is_reported() {
    let mut config = Config::new(Environment::production("PASTE-ORG", "PASTE-BENEFIT"));
    config.log_to_stderr = false;
    let app = Licensing::new(config, temp_dir("unconfigured"));
    assert!(!app.status().configured);
    assert_eq!(
        run(app.activate("ANY")).unwrap_err().kind,
        ErrorKind::NotConfigured
    );
}

#[test]
fn second_computer_is_refused_and_first_stays_licensed() {
    let polar = FakePolar::start();
    polar.issue("KEY-SHARE-AAAAAA", BENEFIT, 1);
    let mac = polar.computer(&temp_dir("share-mac"), MAC, true);
    let laptop = polar.computer(&temp_dir("share-laptop"), LAPTOP, true);
    run(mac.activate("KEY-SHARE-AAAAAA")).unwrap();

    let e = run(laptop.activate("KEY-SHARE-AAAAAA")).unwrap_err();
    assert_eq!(
        e.kind,
        ErrorKind::InUseElsewhere,
        "reclaim must not take another computer's slot"
    );
    assert!(mac.status().licensed);
    assert_eq!(polar.activation_count("KEY-SHARE-AAAAAA"), 1);
}

#[test]
fn copied_license_file_does_not_work_on_another_computer() {
    let polar = FakePolar::start();
    polar.issue("KEY-COPY-AAAAAA", BENEFIT, 1);
    let dir = temp_dir("copy");
    run(polar.computer(&dir, MAC, true).activate("KEY-COPY-AAAAAA")).unwrap();
    assert!(!polar.computer(&dir, LAPTOP, true).status().licensed);
}

#[test]
fn moves_to_a_new_computer_with_release() {
    let polar = FakePolar::start();
    polar.issue("KEY-MOVE-AAAAAA", BENEFIT, 1);
    let mac = polar.computer(&temp_dir("move-mac"), MAC, true);
    let laptop = polar.computer(&temp_dir("move-laptop"), LAPTOP, true);
    run(mac.activate("KEY-MOVE-AAAAAA")).unwrap();

    run(mac.release()).expect("release");
    assert!(!mac.status().licensed);
    assert_eq!(polar.activation_count("KEY-MOVE-AAAAAA"), 0);
    assert!(run(laptop.activate("KEY-MOVE-AAAAAA")).unwrap().licensed);
    assert_eq!(
        run(mac.release()).unwrap_err().kind,
        ErrorKind::NothingToRelease
    );
}

#[test]
fn lost_license_file_is_recovered_by_reclaim() {
    let polar = FakePolar::start();
    polar.issue("KEY-LOST-AAAAAA", BENEFIT, 1);
    let dir = temp_dir("lost");
    let mac = polar.computer(&dir, MAC, true);
    run(mac.activate("KEY-LOST-AAAAAA")).unwrap();

    std::fs::remove_file(mac.license_path()).unwrap(); // app data wiped
    assert!(!mac.status().licensed);
    assert!(
        run(mac.activate("KEY-LOST-AAAAAA"))
            .expect("reclaimed")
            .licensed
    );
    assert_eq!(
        polar.activation_count("KEY-LOST-AAAAAA"),
        1,
        "old released, new made"
    );
}

#[test]
fn lost_license_file_without_worker_needs_a_manual_release() {
    let polar = FakePolar::start();
    polar.issue("KEY-NOWORKER-AA", BENEFIT, 1);
    let mac = polar.computer(&temp_dir("noworker"), MAC, false);
    run(mac.activate("KEY-NOWORKER-AA")).unwrap();
    std::fs::remove_file(mac.license_path()).unwrap();
    assert_eq!(
        run(mac.activate("KEY-NOWORKER-AA")).unwrap_err().kind,
        ErrorKind::InUseElsewhere
    );
}

#[test]
fn disabled_then_re_enabled_key_works_again() {
    let polar = FakePolar::start();
    polar.issue("KEY-DISABLE-AAAA", BENEFIT, 1);
    let mac = polar.computer(&temp_dir("disable"), MAC, true);
    run(mac.activate("KEY-DISABLE-AAAA")).unwrap();

    polar.set_status("KEY-DISABLE-AAAA", Status::Disabled);
    assert_eq!(run(mac.launch_check()).state, LaunchState::Deactivated);
    assert!(!mac.status().licensed, "gate at next launch");
    assert_eq!(
        run(mac.activate("KEY-DISABLE-AAAA")).unwrap_err().kind,
        ErrorKind::Deactivated
    );

    polar.set_status("KEY-DISABLE-AAAA", Status::Granted);
    assert!(
        run(mac.activate("KEY-DISABLE-AAAA"))
            .expect("works again")
            .licensed
    );
    assert_eq!(
        polar.activation_count("KEY-DISABLE-AAAA"),
        1,
        "reused its own activation"
    );
}

#[test]
fn revoked_key_stays_dead_even_after_enable_or_rotate() {
    let polar = FakePolar::start();
    polar.issue("KEY-REVOKE-AAAA", BENEFIT, 1);
    let mac = polar.computer(&temp_dir("revoke"), MAC, true);
    run(mac.activate("KEY-REVOKE-AAAA")).unwrap();

    polar.set_status("KEY-REVOKE-AAAA", Status::Revoked);
    let check = run(mac.launch_check());
    assert_eq!(check.state, LaunchState::Deactivated);
    assert!(!mac.status().licensed);
    assert_eq!(
        run(mac.activate("KEY-REVOKE-AAAA")).unwrap_err().kind,
        ErrorKind::Deactivated
    );

    polar.set_status("KEY-REVOKE-AAAA", Status::Granted); // has no effect on a revoked key
    assert_eq!(
        run(mac.activate("KEY-REVOKE-AAAA")).unwrap_err().kind,
        ErrorKind::Deactivated
    );

    polar.rotate("KEY-REVOKE-AAAA", "KEY-ROTATED-BBBB");
    let e = run(mac.activate("KEY-ROTATED-BBBB")).unwrap_err();
    assert_eq!(
        e.kind,
        ErrorKind::Deactivated,
        "rotating doesn't revive: {}",
        e.message
    );
}

#[test]
fn rotated_key_on_a_lost_file_computer_is_reclaimed() {
    let polar = FakePolar::start();
    polar.issue("KEY-ROT-AAAAAA", BENEFIT, 1);
    let mac = polar.computer(&temp_dir("rotate"), MAC, true);
    run(mac.activate("KEY-ROT-AAAAAA")).unwrap();
    std::fs::remove_file(mac.license_path()).unwrap();
    polar.rotate("KEY-ROT-AAAAAA", "KEY-ROT-NEWNEW");
    assert!(
        run(mac.activate("KEY-ROT-NEWNEW"))
            .expect("same computer, new string")
            .licensed
    );
    assert_eq!(polar.activation_count("KEY-ROT-NEWNEW"), 1);
}

#[test]
fn activation_limit_above_one() {
    let polar = FakePolar::start();
    polar.issue("KEY-THREE-AAAAA", BENEFIT, 3);
    for (i, m) in ["1".repeat(64), "2".repeat(64), "3".repeat(64)]
        .iter()
        .enumerate()
    {
        let c = polar.computer(&temp_dir(&format!("three-{i}")), m, true);
        assert!(run(c.activate("KEY-THREE-AAAAA")).unwrap().licensed);
    }
    let fourth = polar.computer(&temp_dir("three-4"), &"4".repeat(64), true);
    assert_eq!(
        run(fourth.activate("KEY-THREE-AAAAA")).unwrap_err().kind,
        ErrorKind::InUseElsewhere
    );
}
