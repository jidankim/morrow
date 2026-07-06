# iPhone Physical Device Source-Build Testing

This guide is for a technical tester who wants to run Morrow on their own physically connected iPhone without a paid Apple Developer Program membership.

This is not an AirDrop or packaged-IPA flow. The tester builds from source on their own Mac, signs with their own Apple ID in Xcode, and deploys directly to their connected iPhone. The installed app is local to that tester/device, may expire quickly with free provisioning, and must not be treated as a redistributable diagnostic artifact.

## What This Proves

This path can prove:

- The iOS project builds on the tester's Mac.
- Xcode/free provisioning can install Morrow on one connected iPhone.
- The app launches and renders on a real iPhone.
- Basic iPhone UI navigation and startup error states are visible on-device.

This path does not prove:

- Ad hoc, TestFlight, App Store, or shared IPA distribution.
- Apple Developer Program signing or provisioning.
- Passive iPhone Messages access.
- macOS Messages-to-Calendar parity.
- Codex CLI-backed provider behavior on iPhone.
- Full EventKit runtime parity until iOS-specific calendar/reminder behavior is explicitly implemented and tested.

Current iPhone testing should expect some desktop-only functionality to be unavailable. Morrow's macOS Messages discovery and local Codex CLI assumptions do not automatically transfer to iOS.

## Tester Prerequisites

The tester needs:

- A Mac with full Xcode installed.
- An Apple ID signed in to Xcode.
- A physically connected iPhone.
- Node/npm and Rust installed.
- Project helper tools used by Tauri iOS generation: XcodeGen, CocoaPods, and libimobiledevice.

If Xcode prompts for first-launch setup, complete it before building. If the Mac has both Command Line Tools and full Xcode, select full Xcode:

```bash
sudo xcode-select -s /Applications/Xcode.app/Contents/Developer
sudo xcodebuild -runFirstLaunch
```

Install common helper tools with Homebrew if they are missing:

```bash
brew install xcodegen cocoapods libimobiledevice
```

## Prepare The iPhone

1. Connect the iPhone to the Mac with USB.
2. Unlock the iPhone.
3. Tap **Trust This Computer** if prompted.
4. Keep the iPhone unlocked during the first deploy.
5. If Xcode asks for Developer Mode, enable it on the iPhone in **Settings > Privacy & Security > Developer Mode**, restart the phone, then reconnect it.

## Build From Source

From the repository root:

```bash
npm install
npm run tauri:ios:init
```

If Rust reports missing iOS targets, install the requested target and rerun init. For a physical iPhone, the important Rust target is:

```bash
rustup target add aarch64-apple-ios
```

## Option A: Run Through Tauri CLI

This is the fastest path when the device is connected and signing is already usable:

```bash
npm run tauri:ios:dev -- --force-ip-prompt
```

Choose the connected iPhone if prompted. The Tauri dev server must be reachable from the iPhone, so the Mac and iPhone should be on the same network unless Xcode's device tunnel is being used.

If the CLI cannot resolve signing or device selection cleanly, use Option B.

## Option B: Run From Xcode

Open the generated Xcode project:

```bash
open src-tauri/gen/apple/morrow.xcodeproj
```

In Xcode:

1. Select the `morrow_iOS` project and target.
2. Open **Signing & Capabilities**.
3. Check **Automatically manage signing**.
4. Set **Team** to the tester's Apple ID Personal Team.
5. If Xcode rejects `dev.morrow.desktop` as unavailable for that Apple ID, change the Bundle Identifier to a unique value for this local test, such as `dev.<tester-name>.morrow.phone`.
6. Select the connected iPhone as the run destination.
7. Press **Run**.

If Xcode changes generated project settings, treat them as local tester setup. Regenerating the Tauri iOS project may overwrite those local Xcode edits.

## First Launch Trust

If the app installs but iPhone refuses to open it, trust the developer profile:

1. On iPhone, open **Settings**.
2. Go to **General > VPN & Device Management**.
3. Select the developer profile for the tester's Apple ID.
4. Tap **Trust**.
5. Launch Morrow again from Xcode or from the iPhone home screen.

## Tester Smoke Checklist

Record:

- Mac model and macOS version.
- Xcode version.
- iPhone model and iOS version.
- Whether the deploy path was Tauri CLI or Xcode Run.
- Bundle identifier used for signing.
- Whether Developer Mode or profile trust was required.

Then verify:

1. Morrow installs on the iPhone.
2. Morrow launches without crashing.
3. The Morrow UI renders.
4. Status and Settings can be opened.
5. Any startup error or unavailable-state copy is captured exactly.

Expected current result: launch and UI rendering can pass; desktop-only data sources may report unavailable on iPhone.

## Common Failures

**Xcode says the bundle identifier is unavailable**

Use a unique local Bundle Identifier in Xcode's Signing & Capabilities panel. Do not commit generated project changes made only for one tester's Personal Team.

**No signing certificate or team**

Sign in to Xcode with an Apple ID, then choose the Personal Team under Signing & Capabilities.

**Device is not listed**

Unlock the iPhone, trust the Mac, reconnect USB, and check Xcode's Devices and Simulators window.

**Developer Mode required**

Enable Developer Mode on the iPhone, restart, and rerun from Xcode.

**The app expires or stops opening later**

That is expected with free provisioning. Rebuild and redeploy from Xcode.

## References

- Apple: [Running your app on simulated or physical devices](https://developer.apple.com/documentation/xcode/running-your-app-on-simulated-or-physical-devices)
- Tauri: [iOS build CLI options](https://v2.tauri.app/reference/cli/#ios-build)
- Tauri: [iOS signing](https://v2.tauri.app/distribute/sign/ios)
