import Foundation

struct UiMappedMessage {
    let resultTitle: String
    let userReason: String
    let nextAction: String
    let diagnosticCode: String
    let diagnosticDetail: String
    let rawError: String
    let techFields: [String: String]

    var userDetail: String {
        if nextAction.isEmpty {
            return userReason
        }
        if userReason.isEmpty {
            return nextAction
        }
        return "\(userReason)\n\(nextAction)"
    }
}

enum UiErrorMapper {
    static func map(title: String, detail: String, isSuccess: Bool) -> UiMappedMessage {
        let normalized = detail.trimmingCharacters(in: .whitespacesAndNewlines)
        if isSuccess {
            return UiMappedMessage(
                resultTitle: title,
                userReason: normalized,
                nextAction: "",
                diagnosticCode: "",
                diagnosticDetail: "",
                rawError: "",
                techFields: [:]
            )
        }

        let lowered = normalized.lowercased()
        var techFields: [String: String] = [:]
        if let route = extractToken("route", from: normalized) {
            techFields["route"] = route
        }
        if let status = extractToken("status", from: normalized) {
            techFields["status"] = status
        }

        if lowered.contains("single_fallback")
            || lowered.contains("unverified")
            || lowered.contains("invalid_hmac")
            || lowered.contains("route=")
            || lowered.contains("status=") {
            return UiMappedMessage(
                resultTitle: title,
                userReason: Localizer.pick("操作失败。", "The operation failed."),
                nextAction: Localizer.pick("下一步：打开诊断并重试。", "Next: Turn on diagnostics and retry."),
                diagnosticCode: "diag.internal_state",
                diagnosticDetail: normalized,
                rawError: normalized,
                techFields: techFields
            )
        }

        if lowered.contains("no such file") || lowered.contains("not found") || lowered.contains("path") {
            return UiMappedMessage(
                resultTitle: title,
                userReason: Localizer.pick("操作失败。", "The operation failed."),
                nextAction: Localizer.pick("下一步：检查输入路径后重试。", "Next: Check the input path and retry."),
                diagnosticCode: "diag.path",
                diagnosticDetail: normalized,
                rawError: normalized,
                techFields: techFields
            )
        }

        return UiMappedMessage(
            resultTitle: title,
            userReason: Localizer.pick("操作失败。", "The operation failed."),
            nextAction: Localizer.pick("下一步：请重试；若再次失败，请打开诊断。", "Next: Retry, and turn on diagnostics if it fails again."),
            diagnosticCode: "diag.generic",
            diagnosticDetail: normalized,
            rawError: normalized,
            techFields: techFields
        )
    }

    private static func extractToken(_ key: String, from source: String) -> String? {
        let pattern = "\\b\\(key)=([^\\s,;]+)"
        guard let regex = try? NSRegularExpression(pattern: pattern, options: [.caseInsensitive]) else {
            return nil
        }
        let range = NSRange(source.startIndex..<source.endIndex, in: source)
        guard let match = regex.firstMatch(in: source, options: [], range: range),
              let tokenRange = Range(match.range(at: 1), in: source) else {
            return nil
        }
        return String(source[tokenRange])
    }
}
