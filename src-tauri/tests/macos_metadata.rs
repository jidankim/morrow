#[cfg(target_os = "macos")]
use std::{process::Command, str};

#[cfg(target_os = "macos")]
fn codesign_details(binary_path: &str) -> String {
    let output = Command::new("codesign")
        .args(["-dv", "--entitlements", ":-", binary_path])
        .output()
        .expect("codesign should inspect the test binary");
    let mut details = String::from_utf8_lossy(&output.stderr).into_owned();
    details.push_str(&String::from_utf8_lossy(&output.stdout));
    details
}

#[cfg(target_os = "macos")]
fn sign_for_dev_tcc(binary_path: &str) {
    let output = Command::new("codesign")
        .args(["--force", "--sign", "-", binary_path])
        .output()
        .expect("codesign should sign the test binary");
    assert!(
        output.status.success(),
        "codesign failed to sign dev binary: {}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[cfg(target_os = "macos")]
fn embedded_info_plist(binary_path: &str) -> String {
    let output = Command::new("otool")
        .args(["-X", "-s", "__TEXT", "__info_plist", binary_path])
        .output()
        .expect("otool should inspect embedded Info.plist");
    assert!(
        output.status.success(),
        "otool failed to inspect embedded Info.plist: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let hex = String::from_utf8_lossy(&output.stdout);
    let bytes = hex
        .lines()
        .flat_map(|line| line.split_whitespace().skip(1))
        .flat_map(|word| {
            let mut bytes = word
                .as_bytes()
                .chunks(2)
                .filter_map(|chunk| str::from_utf8(chunk).ok())
                .filter_map(|pair| u8::from_str_radix(pair, 16).ok())
                .collect::<Vec<u8>>();
            if bytes.len() == 4 {
                bytes.reverse();
            }
            bytes
        })
        .take_while(|byte| *byte != 0)
        .collect::<Vec<u8>>();
    String::from_utf8(bytes).expect("embedded Info.plist should be UTF-8 XML")
}

#[cfg(target_os = "macos")]
#[test]
fn morrow_binary_embeds_and_signs_eventkit_usage_descriptions() {
    // Given
    let binary_path = env!("CARGO_BIN_EXE_morrow");

    // When
    let plist = embedded_info_plist(binary_path);
    sign_for_dev_tcc(binary_path);
    let codesign = codesign_details(binary_path);

    // Then
    for required_key in [
        "NSCalendarsUsageDescription",
        "NSCalendarsFullAccessUsageDescription",
        "NSRemindersUsageDescription",
        "NSRemindersFullAccessUsageDescription",
    ] {
        assert!(
            plist.contains(required_key),
            "embedded Info.plist should include {required_key}, got:\n{plist}"
        );
    }
    assert!(
        codesign.contains("Identifier=dev.morrow.desktop"),
        "dev binary should use the stable bundle identifier for macOS TCC after dev signing, got:\n{codesign}"
    );
    assert!(
        codesign.contains("Info.plist entries="),
        "dev signing should bind embedded Info.plist entries, got:\n{codesign}"
    );
}
