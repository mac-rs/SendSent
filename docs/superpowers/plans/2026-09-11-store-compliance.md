# Store Compliance Implementation Plan (App Store / Mac App Store / Google Play)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the app acceptable to the iOS App Store, Mac App Store, and Google Play by adding the required plist keys, privacy manifest, sandbox entitlements, permissions, and icons.

**Architecture:** Mostly declarative config: iOS `Info.plist`/`project.yml`, a shared `PrivacyInfo.xcprivacy`, a new macOS `Entitlements.plist`, `tauri.conf.json` bundle settings, and the Android manifest. No runtime logic changes.

**Tech Stack:** Tauri 2, XcodeGen (`project.yml`), Android Gradle.

**Spec:** `docs/superpowers/specs/2026-09-11-store-compliance-design.md`

**Prerequisites (user-supplied, not code):** paid Apple Developer Program team; Apple Distribution cert + App Store profile; Google Play account + upload keystore; public privacy-policy URL.

---

## File Structure

- `src-tauri/gen/apple/project.yml` — iOS/macOS plist properties + privacy manifest source (modify)
- `src-tauri/gen/apple/sendsent_iOS/Info.plist` — iOS keys (modify)
- `src-tauri/gen/apple/PrivacyInfo.xcprivacy` — privacy manifest (create)
- `src-tauri/Entitlements.plist` — macOS sandbox entitlements (create)
- `src-tauri/tauri.conf.json` — bundle category/description/macOS settings/resources (modify)
- `src-tauri/gen/android/app/src/main/AndroidManifest.xml` — permissions (modify)
- `src-tauri/gen/android/app/tauri.properties` — versionCode/versionName (modify)
- `docs/APPSTORE.md` — submission checklist + prerequisites (create)

---

### Task 1: iOS Info.plist keys

**Files:**
- Modify: `src-tauri/gen/apple/sendsent_iOS/Info.plist`
- Modify: `src-tauri/gen/apple/project.yml`

- [ ] **Step 1: Add keys to `Info.plist`**

Inside the top-level `<dict>` add:
```xml
	<key>CFBundleDisplayName</key>
	<string>SendSent</string>
	<key>UIFileSharingEnabled</key>
	<true/>
	<key>LSSupportsOpeningDocumentsInPlace</key>
	<true/>
	<key>ITSAppUsesNonExemptEncryption</key>
	<false/>
	<key>UIApplicationSupportsIndirectInputEvents</key>
	<true/>
```

- [ ] **Step 2: Mirror the keys in `project.yml`**

Under `targets.sendsent_iOS.info.properties` (alongside `NSLocalNetworkUsageDescription`), add:
```yaml
        CFBundleDisplayName: SendSent
        UIFileSharingEnabled: true
        LSSupportsOpeningDocumentsInPlace: true
        ITSAppUsesNonExemptEncryption: false
        UIApplicationSupportsIndirectInputEvents: true
```

- [ ] **Step 3: Lint**

Run: `plutil -lint src-tauri/gen/apple/sendsent_iOS/Info.plist`
Expected: `OK`.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/gen/apple/sendsent_iOS/Info.plist src-tauri/gen/apple/project.yml
git commit -m "feat(ios): App Store Info.plist keys (Files app, encryption, display name)"
```

---

### Task 2: Privacy manifest

**Files:**
- Create: `src-tauri/gen/apple/PrivacyInfo.xcprivacy`
- Modify: `src-tauri/gen/apple/project.yml`
- Modify: `src-tauri/tauri.conf.json` (macOS resource)

- [ ] **Step 1: Create `src-tauri/gen/apple/PrivacyInfo.xcprivacy`**

> These are the common required-reason categories. If an App Store upload is rejected with `ITMS-91053` naming a different API, add that category here with its documented reason code.

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>NSPrivacyTracking</key>
	<false/>
	<key>NSPrivacyCollectedDataTypes</key>
	<array/>
	<key>NSPrivacyAccessedAPITypes</key>
	<array>
		<dict>
			<key>NSPrivacyAccessedAPIType</key>
			<string>NSPrivacyAccessedAPICategoryFileTimestamp</string>
			<key>NSPrivacyAccessedAPITypeReasons</key>
			<array>
				<string>C617.1</string>
			</array>
		</dict>
		<dict>
			<key>NSPrivacyAccessedAPIType</key>
			<string>NSPrivacyAccessedAPICategoryDiskSpace</string>
			<key>NSPrivacyAccessedAPITypeReasons</key>
			<array>
				<string>E174.1</string>
			</array>
		</dict>
		<dict>
			<key>NSPrivacyAccessedAPIType</key>
			<string>NSPrivacyAccessedAPICategorySystemBootTime</string>
			<key>NSPrivacyAccessedAPITypeReasons</key>
			<array>
				<string>35F9.1</string>
			</array>
		</dict>
	</array>
</dict>
</plist>
```

- [ ] **Step 2: Bundle it in the iOS target**

In `project.yml` under `targets.sendsent_iOS.sources`, add an entry:
```yaml
      - path: PrivacyInfo.xcprivacy
        buildPhase: resources
```
Then regenerate the Xcode project:
```bash
pnpm tauri ios init
```
Expected: `PrivacyInfo.xcprivacy` is referenced in `sendsent.xcodeproj/project.pbxproj`.

- [ ] **Step 3: Bundle it in the macOS app**

In `tauri.conf.json`, add to `bundle`:
```json
    "resources": {
      "gen/apple/PrivacyInfo.xcprivacy": "PrivacyInfo.xcprivacy"
    },
```

- [ ] **Step 4: Lint**

Run: `plutil -lint src-tauri/gen/apple/PrivacyInfo.xcprivacy`
Expected: `OK`.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/gen/apple/PrivacyInfo.xcprivacy src-tauri/gen/apple/project.yml \
  src-tauri/gen/apple/sendsent.xcodeproj/project.pbxproj src-tauri/tauri.conf.json
git commit -m "feat(privacy): add PrivacyInfo.xcprivacy for iOS and macOS"
```

---

### Task 3: macOS sandbox entitlements + bundle metadata

**Files:**
- Create: `src-tauri/Entitlements.plist`
- Modify: `src-tauri/tauri.conf.json`

- [ ] **Step 1: Create `src-tauri/Entitlements.plist`**

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>com.apple.security.app-sandbox</key>
	<true/>
	<key>com.apple.security.network.client</key>
	<true/>
	<key>com.apple.security.network.server</key>
	<true/>
	<key>com.apple.security.files.downloads.read-write</key>
	<true/>
	<key>com.apple.security.files.user-selected.read-only</key>
	<true/>
</dict>
</plist>
```

- [ ] **Step 2: Point `tauri.conf.json` at it and set metadata**

Update the `bundle` object:
```json
  "bundle": {
    "active": true,
    "targets": "all",
    "category": "Utility",
    "shortDescription": "Send files and text between nearby devices on your local network.",
    "longDescription": "SendSent discovers nearby devices over Bonjour and transfers files and text peer-to-peer on your LAN.",
    "copyright": "Copyright © 2026 WenQiang Zhao",
    "macOS": {
      "entitlements": "Entitlements.plist",
      "minimumSystemVersion": "10.15"
    },
    "icon": [
      "icons/32x32.png",
      "icons/128x128.png",
      "icons/128x128@2x.png",
      "icons/icon.icns",
      "icons/icon.ico"
    ]
  }
```

- [ ] **Step 3: Lint**

Run: `plutil -lint src-tauri/Entitlements.plist`
Expected: `OK`.

- [ ] **Step 4: Build + smoke test on macOS**

Run: `pnpm tauri build`
Expected: builds; launch the produced `.app`. Verify: app opens, discovers peers, can receive into `~/Downloads/sendsent`, and 「在文件夹中显示」 works under sandbox.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/Entitlements.plist src-tauri/tauri.conf.json
git commit -m "feat(macos): App Sandbox entitlements + bundle metadata"
```

---

### Task 4: iOS/macOS icons without alpha

**Files:**
- Replace: `src-tauri/icons/*` and generated platform assets.

- [ ] **Step 1: Prepare an opaque 1024×1024 source**

Flatten the existing `src-tauri/icons/icon.png` onto an opaque background (iOS icons must not have an alpha channel):
```bash
cd src-tauri/icons
sips -s format png icon.png --out /tmp/icon-src.png
# flatten onto white if it has alpha (ImageMagick example):
magick /tmp/icon-src.png -background white -alpha remove -alpha off /tmp/icon-opaque.png 2>/dev/null || cp /tmp/icon-src.png /tmp/icon-opaque.png
sips -z 1024 1024 /tmp/icon-opaque.png --out /tmp/icon-1024.png
```

- [ ] **Step 2: Regenerate all platform icons**

```bash
pnpm tauri icon /tmp/icon-1024.png
```
Expected: `src-tauri/icons/*` and iOS/Android generated icons refresh.

- [ ] **Step 3: Verify the iOS 1024 icon has no alpha**

```bash
sips -g hasAlpha src-tauri/gen/apple/Assets.xcassets/AppIcon.appiconset/*1024*.png 2>/dev/null || \
  find src-tauri/gen/apple/Assets.xcassets -name "*.png" -exec sips -g hasAlpha {} \; | grep -i "true" && echo "FIX alpha" || echo "no alpha"
```
Expected: no `hasAlpha: true` for the 1024 marketing icon.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/icons src-tauri/gen/apple/Assets.xcassets src-tauri/gen/android/app/src/main/res
git commit -m "chore(icons): opaque app icon set for App Store / Play"
```

---

### Task 5: Android permissions and versioning

**Files:**
- Modify: `src-tauri/gen/android/app/src/main/AndroidManifest.xml`
- Modify: `src-tauri/gen/android/app/tauri.properties`

- [ ] **Step 1: Add networking permissions**

In `AndroidManifest.xml`, after the `INTERNET` permission add:
```xml
    <uses-permission android:name="android.permission.ACCESS_NETWORK_STATE" />
    <uses-permission android:name="android.permission.ACCESS_WIFI_STATE" />
```
If peer discovery fails on Android after this, additionally add `android.permission.CHANGE_WIFI_MULTICAST_STATE` and acquire a multicast lock where the daemon starts.

- [ ] **Step 2: Bump the version for the next upload**

In `src-tauri/gen/android/app/tauri.properties`, set:
```properties
tauri.android.versionCode=1
tauri.android.versionName=0.1.0
```

- [ ] **Step 3: Build an App Bundle**

```bash
export ANDROID_HOME="$HOME/Library/Android/sdk"
export NDK_HOME="$ANDROID_HOME/ndk/27.0.12077973"
pnpm tauri android build --aab
```
Expected: an `.aab` under `src-tauri/gen/android/app/build/outputs/bundle/`.

- [ ] **Step 4: Inspect permissions**

```bash
"$ANDROID_HOME"/build-tools/35.0.0/aapt2 dump permissions \
  src-tauri/gen/android/app/build/outputs/bundle/universalRelease/app-universal-release.aab 2>/dev/null || \
  unzip -p src-tauri/gen/android/app/build/outputs/bundle/universalRelease/app-universal-release.aab base/manifest/AndroidManifest.xml > /dev/null && echo "aab ok"
```
Expected: `INTERNET`, `ACCESS_NETWORK_STATE`, `ACCESS_WIFI_STATE` present.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/gen/android/app/src/main/AndroidManifest.xml src-tauri/gen/android/app/tauri.properties
git commit -m "feat(android): networking permissions + version metadata for Play"
```

---

### Task 6: Submission checklist doc

**Files:**
- Create: `docs/APPSTORE.md`

- [ ] **Step 1: Write `docs/APPSTORE.md`**

```markdown
# Store Submission Checklist

## Prerequisites (account-level)
- [ ] Paid Apple Developer Program membership (team id in `src-tauri/gen/apple/project.yml`).
- [ ] Apple **Distribution** certificate + App Store provisioning profile.
- [ ] macOS: notarization credentials (for Developer ID) OR Mac App Store provisioning.
- [ ] Google Play developer account + upload keystore (write `src-tauri/gen/android/keystore.properties`).
- [ ] Public privacy-policy URL.

## iOS (App Store)
- [ ] `Info.plist`: `UIFileSharingEnabled`, `LSSupportsOpeningDocumentsInPlace`, `ITSAppUsesNonExemptEncryption`, `NSLocalNetworkUsageDescription`, `NSBonjourServices`.
- [ ] `PrivacyInfo.xcprivacy` bundled.
- [ ] AppIcon has no alpha.
- [ ] `pnpm tauri ios build -t aarch64` archives; upload via Transporter.
- [ ] Screenshots + description in App Store Connect.

## macOS (Mac App Store)
- [ ] `Entitlements.plist` (App Sandbox) configured.
- [ ] Archive in Xcode, upload via Transporter.
- [ ] Category = Utility in App Store Connect.

## Google Play
- [ ] Upload `--aab` (not APK).
- [ ] `versionCode` incremented.
- [ ] Data safety form: no data collected/shared.
- [ ] Content rating + privacy policy URL.
```

- [ ] **Step 2: Commit**

```bash
git add docs/APPSTORE.md
git commit -m "docs: App Store / Play submission checklist"
```

---

### Task 7: Final verification

- [ ] **Step 1: Plists**

Run:
```bash
plutil -lint src-tauri/Entitlements.plist \
  src-tauri/gen/apple/PrivacyInfo.xcprivacy \
  src-tauri/gen/apple/sendsent_iOS/Info.plist
```
Expected: all `OK`.

- [ ] **Step 2: iOS archive**

Run: `pnpm tauri ios build -t aarch64`
Expected: BUILD SUCCEEDED; `.ipa` produced.

- [ ] **Step 3: macOS sandbox build**

Run: `pnpm tauri build`
Expected: `.app`/`.dmg` produced; app launches; network + save + reveal work.

- [ ] **Step 4: Android bundle**

Run: `pnpm tauri android build --aab`
Expected: `.aab` produced.

- [ ] **Step 5: Final commit (if any fixes)**

```bash
git add -A
git commit -m "chore: store compliance verification fixes"
```
