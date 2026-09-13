import SwiftUI
import UIKit

// MARK: - 设计系统

enum DS {
    enum Spacing {
        static let xxs: CGFloat = 2
        static let xs: CGFloat = 6
        static let sm: CGFloat = 10
        static let md: CGFloat = 14
        static let lg: CGFloat = 20
        static let xl: CGFloat = 28
    }
    enum Radius {
        static let xs: CGFloat = 8
        static let sm: CGFloat = 12
        static let md: CGFloat = 16
        static let lg: CGFloat = 20
        static let xl: CGFloat = 28
    }
    static let minTap: CGFloat = 44

    enum Anim {
        static let quick: Animation = .easeOut(duration: 0.18)
        static let snappy: Animation = .snappy(duration: 0.30)
        static let smooth: Animation = .smooth(duration: 0.45)
        static let bouncy: Animation = .bouncy(duration: 0.45)
    }
}

// MARK: - 自适应色板(深/浅色自动切换)

extension Color {
    static let ssBg = Color(UIColor { t in
        t.userInterfaceStyle == .dark
            ? UIColor(red: 0.031, green: 0.035, blue: 0.059, alpha: 1)
            : .systemGroupedBackground
    })
    static let ssCard = Color(UIColor { t in
        t.userInterfaceStyle == .dark
            ? UIColor.white.withAlphaComponent(0.05)
            : .secondarySystemGroupedBackground
    })
    static let ssCardBorder = Color(UIColor { t in
        t.userInterfaceStyle == .dark
            ? UIColor.white.withAlphaComponent(0.07)
            : UIColor.separator.withAlphaComponent(0.6)
    })
    static let ssHairline = Color(UIColor { t in
        t.userInterfaceStyle == .dark ? UIColor.white.withAlphaComponent(0.07) : .separator
    })
    static let ssField = Color(UIColor { t in
        t.userInterfaceStyle == .dark ? UIColor.white.withAlphaComponent(0.06) : .tertiarySystemFill
    })
    static let ssIconTile = Color(UIColor { t in
        t.userInterfaceStyle == .dark ? UIColor.white.withAlphaComponent(0.05) : .secondarySystemFill
    })

    static func platformTint(for platform: String) -> Color {
        switch platform {
        case "ios": return .blue
        case "macos": return .indigo
        case "android": return .green
        case "windows": return Color(red: 0, green: 0.47, blue: 0.83)
        case "linux": return .orange
        default: return .gray
        }
    }

    static func avatarFill(for platform: String) -> Color {
        switch platform {
        case "ios": return Color(red: 0.36, green: 0.42, blue: 0.95)
        case "macos": return Color(red: 0.55, green: 0.40, blue: 0.92)
        case "android": return Color(red: 0.18, green: 0.66, blue: 0.42)
        case "windows": return Color(red: 0.0, green: 0.47, blue: 0.84)
        case "linux": return Color(red: 0.85, green: 0.55, blue: 0.13)
        default: return Color(white: 0.55)
        }
    }

    static func fileTint(_ ext: String) -> (bg: Color, fg: Color) {
        func dyn(_ dark: UIColor, _ light: UIColor) -> Color {
            Color(UIColor { $0.userInterfaceStyle == .dark ? dark : light })
        }
        switch ext {
        case "png", "jpg", "jpeg", "gif", "heic", "webp", "tiff", "bmp":
            return (Color.cyan.opacity(0.16), dyn(UIColor(red: 0.49, green: 0.83, blue: 0.99, alpha: 1),
                                                  UIColor(red: 0.02, green: 0.45, blue: 0.72, alpha: 1)))
        case "mp4", "mov", "m4v", "avi", "mkv", "webm":
            return (Color.purple.opacity(0.16), dyn(UIColor(red: 0.85, green: 0.71, blue: 0.99, alpha: 1),
                                                    UIColor(red: 0.49, green: 0.23, blue: 0.80, alpha: 1)))
        case "mp3", "wav", "m4a", "aac", "flac":
            return (Color.pink.opacity(0.16), dyn(UIColor(red: 0.98, green: 0.66, blue: 0.83, alpha: 1),
                                                  UIColor(red: 0.80, green: 0.20, blue: 0.47, alpha: 1)))
        case "pdf":
            return (Color.red.opacity(0.16), dyn(UIColor(red: 0.99, green: 0.64, blue: 0.69, alpha: 1),
                                                 UIColor(red: 0.78, green: 0.16, blue: 0.22, alpha: 1)))
        case "zip", "rar", "7z", "tar", "gz":
            return (Color.orange.opacity(0.16), dyn(UIColor(red: 0.99, green: 0.83, blue: 0.42, alpha: 1),
                                                    UIColor(red: 0.72, green: 0.45, blue: 0.02, alpha: 1)))
        default:
            return (Color.gray.opacity(0.16), dyn(UIColor(red: 0.80, green: 0.84, blue: 0.90, alpha: 1),
                                                  UIColor(red: 0.34, green: 0.38, blue: 0.44, alpha: 1)))
        }
    }
}

// MARK: - 字体

enum Type {
    static func display() -> Font { .system(size: 32, weight: .semibold) }
    static func title() -> Font { .system(size: 22, weight: .semibold) }
    static func titleSmall() -> Font { .system(size: 15, weight: .semibold) }
    static func body() -> Font { .system(size: 16, weight: .regular) }
    static func bodyEm() -> Font { .system(size: 16, weight: .medium) }
    static func callout() -> Font { .system(size: 14, weight: .regular) }
    static func calloutEm() -> Font { .system(size: 14, weight: .medium) }
    static func footnote() -> Font { .system(size: 13, weight: .regular) }
    static func caption() -> Font { .system(size: 11, weight: .regular) }
    static func mono(_ size: CGFloat = 14, weight: Font.Weight = .regular) -> Font {
        .system(size: size, weight: weight, design: .monospaced)
    }
}

// MARK: - 卡片

struct SSCardModifier: ViewModifier {
    var radius: CGFloat = DS.Radius.lg
    func body(content: Content) -> some View {
        content
            .background(RoundedRectangle(cornerRadius: radius, style: .continuous).fill(Color.ssCard))
            .overlay(
                RoundedRectangle(cornerRadius: radius, style: .continuous)
                    .strokeBorder(Color.ssCardBorder, lineWidth: 0.5)
            )
    }
}

extension View {
    func ssCard(_ radius: CGFloat = DS.Radius.lg) -> some View { modifier(SSCardModifier(radius: radius)) }
    func hairline() -> some View { overlay(alignment: .top) { Color.ssHairline.frame(height: 0.5) } }
    @ViewBuilder
    func softScrollEdge(_ edge: Edge.Set = .top) -> some View {
        if #available(iOS 26.0, *) {
            self.scrollEdgeEffectStyle(.soft, for: edge)
        } else { self }
    }
}

// MARK: - 平台

func platformLabel(_ p: String) -> String {
    switch p {
    case "macos": return "macOS"
    case "ios": return "iOS"
    case "android": return "Android"
    case "windows": return "Windows"
    case "linux": return "Linux"
    default: return "Unknown"
    }
}

func platformSymbol(_ p: String) -> String {
    switch p {
    case "ios": return "iphone"
    case "macos": return "laptopcomputer"
    case "android": return "candybarphone"
    case "windows": return "pc"
    case "linux": return "terminal"
    default: return "questionmark"
    }
}

struct PlatformAvatar: View {
    let peer: Peer
    var size: CGFloat = 44
    var showBadge: Bool = true

    private var letter: String { String(peer.name.prefix(1)).uppercased() }
    private var badgeVisible: Bool { showBadge && size >= 28 }

    var body: some View {
        RoundedRectangle(cornerRadius: size * 0.28, style: .continuous)
            .fill(Color.avatarFill(for: peer.platform).gradient)
            .frame(width: size, height: size)
            .overlay(
                Text(letter)
                    .font(.system(size: size * 0.42, weight: .semibold))
                    .foregroundStyle(.white)
            )
            .overlay(alignment: .bottomLeading) {
                if badgeVisible {
                    ZStack {
                        Circle().fill(Color.ssBg)
                        Circle().strokeBorder(Color.ssCardBorder, lineWidth: 0.5)
                        Image(systemName: platformSymbol(peer.platform))
                            .font(.system(size: size * 0.19, weight: .semibold))
                            .foregroundStyle(Color.avatarFill(for: peer.platform))
                    }
                    .frame(width: size * 0.40, height: size * 0.40)
                    .offset(x: -size * 0.06, y: size * 0.06)
                }
            }
            .overlay(alignment: .bottomTrailing) {
                Circle()
                    .fill(.green)
                    .frame(width: size * 0.26, height: size * 0.26)
                    .overlay(Circle().stroke(Color.ssBg, lineWidth: 2))
                    .offset(x: size * 0.06, y: size * 0.06)
            }
    }
}

// MARK: - 传输状态

func statusColor(_ s: String) -> Color {
    switch s {
    case "completed": return .green
    case "rejected": return .orange
    case "cancelled": return .gray
    default: return .red
    }
}

func statusSymbol(_ s: String) -> String {
    switch s {
    case "completed": return "checkmark"
    case "rejected": return "hand.raised.fill"
    case "cancelled": return "slash"
    default: return "exclamationmark"
    }
}

func statusIcon(_ s: String) -> String {
    switch s {
    case "completed": return "checkmark.circle.fill"
    case "rejected": return "hand.raised.circle.fill"
    case "cancelled": return "slash.circle.fill"
    default: return "exclamationmark.circle.fill"
    }
}

func fileSymbol(_ name: String) -> String {
    let ext = (name as NSString).pathExtension.lowercased()
    switch ext {
    case "png", "jpg", "jpeg", "gif", "heic", "webp", "tiff", "bmp": return "photo"
    case "mp4", "mov", "m4v", "avi", "mkv", "webm": return "film"
    case "mp3", "wav", "m4a", "aac", "flac": return "waveform"
    case "pdf": return "doc.richtext"
    case "zip", "rar", "7z", "tar", "gz": return "doc.zipper"
    case "txt", "md", "rtf", "csv": return "doc.text"
    default: return "doc"
    }
}

func formatRelative(_ ms: Int64) -> String {
    guard ms > 0 else { return "" }
    let f = RelativeDateTimeFormatter()
    f.locale = Locale(identifier: "zh_CN")
    f.unitsStyle = .abbreviated
    return f.localizedString(for: Date(timeIntervalSince1970: Double(ms) / 1000), relativeTo: Date())
}

// MARK: - 空状态

struct EmptyStateView: View {
    let icon: String
    let title: String
    var subtitle: String? = nil
    var body: some View {
        VStack(spacing: DS.Spacing.sm) {
            ZStack {
                Circle().fill(Color.ssField).frame(width: 72, height: 72)
                Image(systemName: icon)
                    .font(.system(size: 28, weight: .regular))
                    .foregroundStyle(.secondary)
            }
            Text(title).font(Type.titleSmall()).foregroundStyle(.primary)
            if let subtitle {
                Text(subtitle)
                    .font(Type.callout())
                    .foregroundStyle(.secondary)
                    .multilineTextAlignment(.center)
            }
        }
        .frame(maxWidth: .infinity)
        .padding(.vertical, DS.Spacing.xl)
    }
}

// MARK: - 主题

enum AppTheme: String, CaseIterable, Identifiable {
    case system, light, dark
    var id: String { rawValue }
    var label: String {
        switch self {
        case .system: return "跟随系统"
        case .light: return "浅色"
        case .dark: return "深色"
        }
    }
    var colorScheme: ColorScheme? {
        switch self {
        case .system: return nil
        case .light: return .light
        case .dark: return .dark
        }
    }
}
