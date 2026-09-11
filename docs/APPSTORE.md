# 上架清单（App Store · Mac App Store · Google Play）

## 前置条件（账号/材料，非代码）

- [ ] 付费 **Apple Developer Program** 会员（team id 写在 `src-tauri/gen/apple/project.yml`）。
- [ ] Apple **Distribution** 证书 + App Store 描述文件。
- [ ] macOS：App Store 分发用对应描述文件（Developer ID 分发则另需公证凭据）。
- [ ] **Google Play** 开发者账号 + 上传 keystore（写入 `src-tauri/gen/android/keystore.properties`）。
- [ ] 公网可访问的**隐私政策 URL**。

## iOS（App Store）

- [ ] `Info.plist`：`UIFileSharingEnabled`、`LSSupportsOpeningDocumentsInPlace`、`ITSAppUsesNonExemptEncryption`、`NSLocalNetworkUsageDescription`、`NSBonjourServices`。
- [ ] `PrivacyInfo.xcprivacy` 已打进 bundle。
- [ ] AppIcon **无 alpha 通道**（`sips -g hasAlpha` 全为 no；`pnpm tauri icon` 会输出 RGBA，需再拍平）。
- [ ] `pnpm tauri ios build -t aarch64` 归档成功；用 Transporter 上传。
- [ ] App Store Connect 填截图、描述、分类。

## macOS（Mac App Store）

- [ ] `src-tauri/Entitlements.plist`（App Sandbox）已在 `tauri.conf.json` 的 `bundle.macOS.entitlements` 指向。
- [ ] 本地验证沙盒行为需签名身份；未设置时 `tauri build` 只会 ad-hoc 且**不写 entitlements**：
      `APPLE_SIGNING_IDENTITY="-" pnpm tauri build`（用 `codesign -d --entitlements :-` 校验）。
- [ ] Xcode 归档 → Transporter 上传。
- [ ] App Store Connect 分类 = 工具（Utility）。

## Google Play

- [ ] 上传 **AAB**（`pnpm tauri android build --aab`），不要用 APK。
- [ ] `tauri.android.versionCode` 每次上传递增（`gen/android/app/tauri.properties`）。
- [ ] 权限：`INTERNET`、`ACCESS_NETWORK_STATE`、`ACCESS_WIFI_STATE`；只保留真实使用项。
- [ ] Data safety 表单：声明**不收集/不共享**用户数据（本地发现不外发）。
- [ ] 内容分级 + 隐私政策 URL。

## 常用校验命令

```bash
# plist 语法
plutil -lint src-tauri/Entitlements.plist \
  src-tauri/gen/apple/PrivacyInfo.xcprivacy \
  src-tauri/gen/apple/sendsent_iOS/Info.plist

# iOS 图标无 alpha
for f in src-tauri/gen/apple/Assets.xcassets/AppIcon.appiconset/*.png; do
  sips -g hasAlpha "$f" | grep -q "yes" && echo "ALPHA: $f"
done; echo done

# macOS entitlements（需签名身份）
APPLE_SIGNING_IDENTITY="-" pnpm tauri build
codesign -d --entitlements :- src-tauri/target/release/bundle/macos/sendsent.app
```
