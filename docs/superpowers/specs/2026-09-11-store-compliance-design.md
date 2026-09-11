# 上架合规设计：iOS App Store · Mac App Store · Google Play

日期：2026-09-11
状态：已评审（待用户复核）
关联：`2026-09-11-cross-platform-ui-history-design.md`（iOS Documents 暴露、macOS 保存目录有交叉）

## 目标

让应用在代码与工程配置层面满足三大商店的上架要求，消除常见拒审点。账号、证书、密钥、隐私政策等**材料类前置条件**单独列出（非代码）。

## 非目标（YAGNI）

- 不处理商店后台元数据（截图、宣传文案、分类选择、价格）与商务流程。
- 不引入内购、账号系统、数据上报。
- 不做代码混淆/加固。

## 现状与缺口

| 项 | 现状 | 缺口 |
| --- | --- | --- |
| iOS Info.plist | 有 `NSLocalNetworkUsageDescription`/`NSBonjourServices` | 缺 `UIFileSharingEnabled`、`LSSupportsOpeningDocumentsInPlace`、`ITSAppUsesNonExemptEncryption` |
| Apple 隐私清单 | 无 | 需 `PrivacyInfo.xcprivacy` |
| iOS/macOS 图标 | 模板图标（可能含 alpha） | 需不透明 1024 图标 |
| macOS 沙盒 | 未启用 | 需 App Sandbox + 网络/文件 entitlements |
| Android targetSdk | `36` ✅ | 权限审计 |
| Android 签名 | 无 keystore | 上传密钥 / Play App Signing |
| 隐私政策 | 无 | 需 URL（两个商店都要求） |

---

## 1. Apple 通用

- **隐私清单** `PrivacyInfo.xcprivacy`（iOS 与 macOS target 都包含）：
  - `NSPrivacyTracking = false`。
  - `NSPrivacyCollectedDataTypes = []`（本地网络发现，不上报；如需可加“未收集”声明）。
  - `NSPrivacyAccessedAPITypes`：**审计后按实际使用**声明 required-reason APIs。初步候选：
    - `NSPrivacyAccessedAPICategoryFileTimestamp`（reason `C617.1`）
    - `NSPrivacyAccessedAPICategoryDiskSpace`（reason `E174.1`）
    - `NSPrivacyAccessedAPICategorySystemBootTime`（reason `35F9.1`）
  - 目标：审核不被 `ITMS-91053`（缺 API 声明）拦。
- **出口合规**：`ITSAppUsesNonExemptEncryption = false`（rustls/TLS 属标准加密豁免）。
- **图标**：iOS/macOS 图标**不得含 alpha 通道**；提供 1024×1024 不透明版本。
- **版本**：`CFBundleShortVersionString` 语义化；`CFBundleVersion` 每次提交递增。
- **隐私政策 URL**：需公网可访问（材料）。
- **签名（前置）**：付费 Apple Developer Program 账号、Apple **Distribution** 证书、App Store provisioning profile。

## 2. iOS（App Store）

`src-tauri/gen/apple/sendsent_iOS/Info.plist` 增加：

- `UIFileSharingEnabled = true`、`LSSupportsOpeningDocumentsInPlace = true` → 接收目录 `Documents/sendsent` 在系统「文件」App 可见（同时解决“传输后找不到文件”）。
- `ITSAppUsesNonExemptEncryption = false`。
- `CFBundleDisplayName`（如 `SendSent`）、`UIApplicationSupportsIndirectInputEvents = true`。
- 保持 `NSLocalNetworkUsageDescription` + `NSBonjourServices: [_sendsent._tcp]`。
- 不使用 multicast entitlement（走系统 Bonjour，规避受限 entitlement 审核）。
- 图标：`Assets.xcassets/AppIcon` 全员无 alpha。
- 基础可达性：触控目标 ≥44pt、对比度；动态字体（尽量）。

## 3. macOS（Mac App Store）

- 新建 `src-tauri/Entitlements.plist`，`tauri.conf.json` 的 `bundle.macOS.entitlements` 指向它：
  - `com.apple.security.app-sandbox = true`
  - `com.apple.security.network.client = true`
  - `com.apple.security.network.server = true`（监听 + Bonjour 广播/浏览）
  - `com.apple.security.files.downloads.read-write = true`（保存到 `~/Downloads/sendsent`）
  - `com.apple.security.files.user-selected.read-only = true`（用户选择的待发送文件）
- 保存目录维持 `~/Downloads/sendsent`（沙盒下由 downloads entitlement 授权）。
- `bundle.category = "public.app-category.utilities"`、`minimumSystemVersion`。
- 「在文件夹中显示 / 打开文件夹」在沙盒内仅对可访问路径生效，需实测。
- 分发：Xcode 归档 → App Store Connect（Transporter）。

## 4. Android（Google Play）

- **App Bundle**：以 `pnpm tauri android build --aab` 产出 `.aab` 上传（Play 要求），APK 仅本地测试。
- `targetSdk = 36` ✅（满足新目标）；`minSdk = 24`。
- **权限审计**：`AndroidManifest.xml` 当前仅 `INTERNET`。若 mDNS/网络状态检测实际使用，补声明：
  - `ACCESS_NETWORK_STATE`、`ACCESS_WIFI_STATE`
  - 如自实现组播锁需 `CHANGE_WIFI_MULTICAST_STATE`（当前用 mdns-sd，实测需要再补）。
  - 原则：只声明真实使用的权限。
- **签名（前置）**：上传 keystore + Play App Signing；`versionCode` 每次上传递增。
- **Data safety 表单**：声明「不收集/不共享用户数据」；本地网络发现不出设备。
- **隐私政策 URL**（材料）。
- 自适应图标（adaptive icon）无透明问题；含 `arm64-v8a`（64 位）。

---

## 前置条件（非代码，需你提供/确认）

1. 现有 Apple team `F8JZTX6J52` 是否为**付费** Apple Developer Program 团队？（个人免费团队无法上架）
2. Apple Distribution 证书 + App Store provisioning profile。
3. Google Play 开发者账号 + 上传 keystore（含密码，配置到 `keystore.properties`）。
4. 隐私政策 URL。

> 这些不满足时，代码/配置全部就绪也无法完成上架；我会在实现中把需要填的值留成清晰的配置点。

## 验证

- macOS：`pnpm tauri build` 启用沙盒 entitlements 后能启动；接收保存到 `~/Downloads/sendsent`、reveal/open 正常、发现与传输可用。
- iOS：`pnpm tauri ios build -t aarch64` 归档通过；`plutil -lint` Info.plist；确认 `PrivacyInfo.xcprivacy` 与无 alpha 图标被打包；「文件」App 能看到 `sendsent` 目录。
- Android：`pnpm tauri android build --aab` 通过；`aapt dump permissions` 核对权限；确认 aab 含 arm64-v8a。
- 通用：`plutil -lint` 各 plist、`cargo clippy -D warnings`、`pnpm build`。

## 风险

- required-reason API 清单若不准确会被审核自动拦（`ITMS-91053`）。
- Mac App Store 沙盒可能影响 open/reveal 与保存路径，需在真机验证。
- 无付费开发者账号时无法上架（首要阻塞）。
- 图标 alpha、动态字体等细节易被忽略，需专门检查。
