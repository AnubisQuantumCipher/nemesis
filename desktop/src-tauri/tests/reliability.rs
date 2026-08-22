//! Reliability, recovery, and data-integrity gate (Phase F).
//!
//! Every test drives a real product seam through the public desktop API and
//! asserts the failure lands at its intended semantic boundary (F-14). No test
//! accepts a syntax error, panic, or absent fixture as negative evidence.

use std::fs;
use std::os::unix::fs::{PermissionsExt, symlink};
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use nemesis_desktop::{
    DesktopSettings, TextScale, initialize_local_home, load_last_mission, load_settings,
    save_settings,
};

fn test_directory(name: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    PathBuf::from("/tmp").join(format!("nemesis-{name}-{}-{nonce}", std::process::id()))
}

fn home_json(schema_version: u64) -> String {
    format!("{{\"schema\":\"nemesis.local-home/v1\",\"schemaVersion\":{schema_version}}}\n")
}

// F-02 / F-04: a missing last-mission pointer is a clean cold start, not an error.
#[test]
fn missing_last_mission_pointer_is_a_clean_cold_start() {
    let home = test_directory("cold-start");
    initialize_local_home(&home).unwrap();
    assert!(load_last_mission(&home).unwrap().is_none());
    let _ = fs::remove_dir_all(&home);
}

// F-04: a truncated / malformed last-mission pointer is refused, never half-loaded.
#[test]
fn corrupt_last_mission_pointer_is_refused() {
    let home = test_directory("corrupt-pointer");
    initialize_local_home(&home).unwrap();
    let pointer = home.join("last-mission.json");

    fs::write(&pointer, b"{ this is not json").unwrap();
    assert!(load_last_mission(&home).is_err());

    fs::write(
        &pointer,
        br#"{"schema":"nemesis.last-mission/v1","missionId":"mis_short","resultPath":"missions/x.json"}"#,
    )
    .unwrap();
    let error = load_last_mission(&home).unwrap_err();
    assert!(format!("{error}").contains("invalid"), "{error}");

    let _ = fs::remove_dir_all(&home);
}

// F-04: a symlinked or oversized pointer is refused before any read of its target.
#[test]
fn symlinked_or_oversized_last_mission_pointer_is_refused() {
    let home = test_directory("pointer-shape");
    initialize_local_home(&home).unwrap();
    let pointer = home.join("last-mission.json");

    let elsewhere = home.join("elsewhere.json");
    fs::write(&elsewhere, b"{}").unwrap();
    symlink(&elsewhere, &pointer).unwrap();
    let error = load_last_mission(&home).unwrap_err();
    assert!(
        format!("{error}").contains("bounded regular file"),
        "{error}"
    );
    fs::remove_file(&pointer).unwrap();

    fs::write(&pointer, vec![b'x'; 5000]).unwrap();
    let error = load_last_mission(&home).unwrap_err();
    assert!(
        format!("{error}").contains("bounded regular file"),
        "{error}"
    );

    let _ = fs::remove_dir_all(&home);
}

// F-05: a corrupt home manifest is refused and the on-disk bytes are not mutated.
#[test]
fn corrupt_home_manifest_is_refused_without_mutation() {
    let home = test_directory("corrupt-home");
    initialize_local_home(&home).unwrap();
    let manifest = home.join("home.json");

    let poison = b"{ not valid json at all";
    fs::write(&manifest, poison).unwrap();
    assert!(initialize_local_home(&home).is_err());
    assert_eq!(
        fs::read(&manifest).unwrap(),
        poison,
        "a refused init must not rewrite the manifest"
    );

    let _ = fs::remove_dir_all(&home);
}

// F-05 / F-12: a future-schema home is refused (downgrade / rollback safety).
#[test]
fn future_schema_home_is_refused_as_downgrade_guard() {
    let home = test_directory("future-home");
    initialize_local_home(&home).unwrap();
    fs::write(home.join("home.json"), home_json(2)).unwrap();
    let error = initialize_local_home(&home).unwrap_err();
    assert!(
        format!("{error}").contains("unsupported local home schema"),
        "{error}"
    );
    let _ = fs::remove_dir_all(&home);
}

// F-04: a symlinked or oversized manifest is refused before deserialization.
#[test]
fn symlinked_or_oversized_home_manifest_is_refused() {
    let home = test_directory("home-shape");
    initialize_local_home(&home).unwrap();
    let manifest = home.join("home.json");

    fs::remove_file(&manifest).unwrap();
    let target = home.join("target.json");
    fs::write(&target, home_json(1)).unwrap();
    symlink(&target, &manifest).unwrap();
    let error = initialize_local_home(&home).unwrap_err();
    assert!(
        format!("{error}").contains("bounded regular file"),
        "{error}"
    );
    fs::remove_file(&manifest).unwrap();

    fs::write(&manifest, vec![b' '; 5000]).unwrap();
    let error = initialize_local_home(&home).unwrap_err();
    assert!(
        format!("{error}").contains("bounded regular file"),
        "{error}"
    );

    let _ = fs::remove_dir_all(&home);
}

// F-02: settings recover to safe defaults when absent, and refuse when corrupt.
#[test]
fn settings_recover_or_refuse_at_the_intended_seam() {
    let home = test_directory("settings-recovery");
    initialize_local_home(&home).unwrap();

    // Absent -> safe default (reduce_motion defaults on).
    let defaults = load_settings(&home).unwrap();
    assert_eq!(defaults, DesktopSettings::default());
    assert!(defaults.reduce_motion);

    let settings = home.join("settings.json");

    fs::write(&settings, b"{ broken").unwrap();
    assert!(load_settings(&home).is_err());

    fs::write(
        &settings,
        br#"{"textScale":"standard","reduceMotion":false,"rogue":1}"#,
    )
    .unwrap();
    assert!(
        load_settings(&home).is_err(),
        "unknown fields must be refused"
    );

    fs::remove_file(&settings).unwrap();
    let target = home.join("target-settings.json");
    fs::write(&target, br#"{"textScale":"standard","reduceMotion":true}"#).unwrap();
    symlink(&target, &settings).unwrap();
    let error = load_settings(&home).unwrap_err();
    assert!(
        format!("{error}").contains("bounded regular file"),
        "{error}"
    );

    let _ = fs::remove_dir_all(&home);
}

// F-02: settings persist across a simulated relaunch and are reversible.
#[test]
fn settings_persist_and_reverse_across_relaunch() {
    let home = test_directory("settings-persist");
    initialize_local_home(&home).unwrap();

    let large = DesktopSettings {
        text_scale: TextScale::Large,
        reduce_motion: false,
    };
    save_settings(&home, &large).unwrap();

    // Simulated relaunch: fresh init over an existing home, then reload.
    initialize_local_home(&home).unwrap();
    assert_eq!(load_settings(&home).unwrap(), large);

    save_settings(&home, &DesktopSettings::default()).unwrap();
    assert_eq!(load_settings(&home).unwrap(), DesktopSettings::default());

    let _ = fs::remove_dir_all(&home);
}

// F-10: crash-orphaned atomic-write temporaries are swept on init; unrelated
// dotfiles and non-temp names are preserved.
#[test]
fn stale_atomic_write_temporaries_are_swept_on_init() {
    let home = test_directory("temp-sweep");
    initialize_local_home(&home).unwrap();

    let orphan_root = home.join(".settings.json.tmp-999999");
    let orphan_child = home.join("missions").join(".result.json.tmp-4242");
    let unrelated_dot = home.join(".keepme");
    let non_temp = home.join("settings.json.tmp-5"); // no leading dot -> not swept
    fs::write(&orphan_root, b"partial").unwrap();
    fs::write(&orphan_child, b"partial").unwrap();
    fs::write(&unrelated_dot, b"keep").unwrap();
    fs::write(&non_temp, b"keep").unwrap();

    initialize_local_home(&home).unwrap();

    assert!(!orphan_root.exists(), "root orphan temp must be swept");
    assert!(!orphan_child.exists(), "subdir orphan temp must be swept");
    assert!(
        unrelated_dot.exists(),
        "unrelated dotfile must be preserved"
    );
    assert!(non_temp.exists(), "non-temp name must be preserved");
    assert!(
        home.join("home.json").exists(),
        "manifest must survive sweep"
    );

    let _ = fs::remove_dir_all(&home);
}

// F-10: a leftover foreign-pid temporary never blocks a subsequent commit.
#[test]
fn leftover_temporary_does_not_block_commit() {
    let home = test_directory("temp-nonblock");
    initialize_local_home(&home).unwrap();

    let leftover = home.join(".settings.json.tmp-123456");
    fs::write(&leftover, b"stale").unwrap();

    let wanted = DesktopSettings {
        text_scale: TextScale::Large,
        reduce_motion: false,
    };
    save_settings(&home, &wanted).unwrap();
    assert_eq!(load_settings(&home).unwrap(), wanted);

    let _ = fs::remove_dir_all(&home);
}

// F-07: a read-only parent surfaces a clean storage error, never a panic.
#[test]
fn read_only_parent_fails_cleanly() {
    if unsafe { libc_geteuid() } == 0 {
        return; // root ignores mode bits; skip rather than assert falsely
    }
    let base = test_directory("readonly-parent");
    fs::create_dir_all(&base).unwrap();
    fs::set_permissions(&base, fs::Permissions::from_mode(0o500)).unwrap();

    let home = base.join("home");
    let result = initialize_local_home(&home);
    fs::set_permissions(&base, fs::Permissions::from_mode(0o700)).unwrap();
    assert!(
        result.is_err(),
        "read-only parent must refuse home creation"
    );

    let _ = fs::remove_dir_all(&base);
}

// F-07: a full filesystem fails the write but preserves the prior committed value.
#[test]
fn disk_full_write_preserves_prior_settings() {
    let base = test_directory("disk-full");
    fs::create_dir_all(&base).unwrap();
    let image = base.join("vol.dmg");
    let mount = base.join("mnt");
    fs::create_dir_all(&mount).unwrap();

    if !hdiutil(&[
        "create",
        "-size",
        "8m",
        "-fs",
        "HFS+",
        "-volname",
        "NEM",
        "-quiet",
        image.to_str().unwrap(),
    ]) {
        let _ = fs::remove_dir_all(&base);
        return; // environment without hdiutil -> skip, do not false-fail
    }
    if !hdiutil(&[
        "attach",
        "-nobrowse",
        "-quiet",
        "-mountpoint",
        mount.to_str().unwrap(),
        image.to_str().unwrap(),
    ]) {
        let _ = fs::remove_dir_all(&base);
        return;
    }

    let home = mount.join("home");
    initialize_local_home(&home).unwrap();
    let good = DesktopSettings {
        text_scale: TextScale::Large,
        reduce_motion: true,
    };
    save_settings(&home, &good).unwrap();

    // Exhaust free space progressively: coarse chunks first, then ever-finer
    // writes so no slack remains for even a ~50-byte settings file.
    let mut filler = 0usize;
    for chunk_size in [64 * 1024usize, 4 * 1024, 256, 16] {
        let chunk = vec![0u8; chunk_size];
        loop {
            let path = mount.join(format!("fill-{filler}"));
            if fs::write(&path, &chunk).is_err() {
                break;
            }
            filler += 1;
            if filler > 200_000 {
                break; // safety bound
            }
        }
    }

    let attempt = save_settings(
        &home,
        &DesktopSettings {
            text_scale: TextScale::Standard,
            reduce_motion: false,
        },
    );
    assert!(attempt.is_err(), "write on a full volume must fail");
    assert_eq!(
        load_settings(&home).unwrap(),
        good,
        "prior committed settings must survive a failed write"
    );

    hdiutil(&["detach", "-quiet", mount.to_str().unwrap()]);
    let _ = fs::remove_dir_all(&base);
}

// F-11: repeated launch/settings cycles are stable (no drift, no leak of state).
#[test]
fn repeated_launch_cycles_are_stable() {
    let home = test_directory("soak");
    let first = initialize_local_home(&home).unwrap();
    assert!(first.first_launch);
    let manifest_bytes = fs::read(home.join("home.json")).unwrap();

    for cycle in 0..25 {
        let status = initialize_local_home(&home).unwrap();
        assert!(
            !status.first_launch,
            "cycle {cycle} must not re-declare first launch"
        );
        assert_eq!(
            fs::read(home.join("home.json")).unwrap(),
            manifest_bytes,
            "manifest must be byte-stable across cycles"
        );
        let settings = DesktopSettings {
            text_scale: if cycle % 2 == 0 {
                TextScale::Large
            } else {
                TextScale::Standard
            },
            reduce_motion: cycle % 3 == 0,
        };
        save_settings(&home, &settings).unwrap();
        assert_eq!(load_settings(&home).unwrap(), settings);
    }

    let _ = fs::remove_dir_all(&home);
}

// F-12: fresh install, then uninstall-by-deletion, then clean reinstall.
#[test]
fn install_uninstall_reinstall_cycle_is_clean() {
    let home = test_directory("lifecycle");

    let install = initialize_local_home(&home).unwrap();
    assert!(
        install.first_launch,
        "v0.1.0 had no local home; successor first run is a fresh install"
    );
    save_settings(
        &home,
        &DesktopSettings {
            text_scale: TextScale::Large,
            reduce_motion: false,
        },
    )
    .unwrap();

    // Uninstall with data deletion.
    fs::remove_dir_all(&home).unwrap();
    assert!(!home.exists());

    // Reinstall is a clean first launch with defaulted settings.
    let reinstall = initialize_local_home(&home).unwrap();
    assert!(reinstall.first_launch);
    assert_eq!(load_settings(&home).unwrap(), DesktopSettings::default());

    let _ = fs::remove_dir_all(&home);
}

fn hdiutil(arguments: &[&str]) -> bool {
    Command::new("/usr/bin/hdiutil")
        .args(arguments)
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

unsafe extern "C" {
    #[link_name = "geteuid"]
    fn libc_geteuid() -> u32;
}
