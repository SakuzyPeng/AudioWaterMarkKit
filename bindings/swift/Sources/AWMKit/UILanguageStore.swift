import Foundation
import CAWMKit

public enum AWMUILanguage: String, CaseIterable, Identifiable {
    case zhCN = "zh-CN"
    case enUS = "en-US"

    public var id: String { rawValue }
}

public enum AWMUILanguageStore {
    public static func get() throws -> AWMUILanguage? {
        let value = try fetchCString { out, outLen, required in
            awm_ui_language_get(out, outLen, required)
        }
        guard !value.isEmpty else { return nil }
        return AWMUILanguage(rawValue: value)
    }

    public static func set(_ language: AWMUILanguage?) throws {
        let code: Int32
        if let language {
            code = language.rawValue.withCString { ptr in
                awm_ui_language_set(ptr)
            }
        } else {
            code = awm_ui_language_set(nil)
        }

        guard code == AWM_SUCCESS.rawValue else {
            throw AWMError(code: code)
        }
    }

    private static func fetchCString(
        _ caller: (_ out: UnsafeMutablePointer<CChar>?, _ outLen: Int, _ required: UnsafeMutablePointer<Int>?) -> Int32
    ) throws -> String {
        var requiredLen = 0
        var code = caller(nil, 0, &requiredLen)
        guard code == AWM_SUCCESS.rawValue else {
            throw AWMError(code: code)
        }

        let size = max(requiredLen, 1)
        var buffer = [CChar](repeating: 0, count: size)
        code = caller(&buffer, buffer.count, &requiredLen)
        guard code == AWM_SUCCESS.rawValue else {
            throw AWMError(code: code)
        }
        return String(cString: buffer)
    }
}
