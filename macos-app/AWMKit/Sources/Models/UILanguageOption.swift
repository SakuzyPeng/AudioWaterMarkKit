import Foundation

enum UILanguageOption: String, CaseIterable, Identifiable {
    case zhCN = "zh-CN"
    case enUS = "en-US"

    var id: String { rawValue }

    var displayName: String {
        switch self {
        case .zhCN:
            return "中文"
        case .enUS:
            return "English"
        }
    }

    var locale: Locale {
        Locale(identifier: rawValue)
    }

    static func defaultFromSystem() -> UILanguageOption {
        let preferred = Locale.preferredLanguages.first?.lowercased() ?? ""
        return preferred.hasPrefix("zh") ? .zhCN : .enUS
    }
}
