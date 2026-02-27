import Foundation
import AWMKit

enum Localizer {
    private static let table = "Localizable"
    private static let fallbackValues: [String: (zh: String, en: String)] = [
        "ui.sidebar.appearance": ("外观", "Appearance"),
        "ui.sidebar.appearance.help": ("切换应用外观", "Switch app appearance")
    ]

    static func tr(_ key: String, _ args: CVarArg...) -> String {
        tr(key, args)
    }

    static func tr(_ key: String, _ args: [CVarArg]) -> String {
        let selected = currentLanguage()
        let format = lookup(key: key, language: selected)
            ?? lookup(key: key, language: .enUS)
            ?? lookup(key: key, language: .zhCN)
            ?? fallback(key: key, language: selected)
            ?? key
        guard !args.isEmpty else {
            return format
        }
        return String(format: format, locale: selected.locale, arguments: args)
    }

    // Transitional helper to centralize language selection while migrating hardcoded pairs to keys.
    static func pick(_ zh: String, _ en: String) -> String {
        currentLanguage() == .enUS ? en : zh
    }

    private static func currentLanguage() -> UILanguageOption {
        if let stored = try? AWMUILanguageStore.get(), let resolved = UILanguageOption(rawValue: stored.rawValue) {
            return resolved
        }
        return UILanguageOption.defaultFromSystem()
    }

    private static func lookup(key: String, language: UILanguageOption) -> String? {
        guard let bundle = bundle(for: language) else {
            return nil
        }
        let value = bundle.localizedString(forKey: key, value: nil, table: table)
        if value == key {
            return nil
        }
        return value
    }

    private static func bundle(for language: UILanguageOption) -> Bundle? {
        let identifiers: [String]
        switch language {
        case .enUS:
            identifiers = ["en-US", "en"]
        case .zhCN:
            identifiers = ["zh-Hans", "zh-CN", "zh"]
        }

        for identifier in identifiers {
            if let path = Bundle.main.path(forResource: identifier, ofType: "lproj"),
               let bundle = Bundle(path: path) {
                return bundle
            }
        }
        return Bundle.main
    }

    private static func fallback(key: String, language: UILanguageOption) -> String? {
        guard let value = fallbackValues[key] else {
            return nil
        }
        return language == .enUS ? value.en : value.zh
    }
}
