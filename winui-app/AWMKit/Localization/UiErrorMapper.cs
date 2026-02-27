using System;
using System.Collections.Generic;
using System.Linq;
using System.Text.RegularExpressions;

namespace AWMKit.Localization;

public readonly record struct UiMappedMessage(
    string ResultTitle,
    string UserReason,
    string NextAction,
    string DiagnosticCode,
    string DiagnosticDetail,
    string RawError,
    string TechFields)
{
    public string UserDetail
    {
        get
        {
            if (string.IsNullOrWhiteSpace(NextAction))
            {
                return UserReason;
            }

            if (string.IsNullOrWhiteSpace(UserReason))
            {
                return NextAction;
            }

            return $"{UserReason}\n{NextAction}";
        }
    }
}

public static class UiErrorMapper
{
    private static readonly Regex TechTokenRegex = new(@"\b(route|status)=[^\s,;]+", RegexOptions.IgnoreCase | RegexOptions.Compiled);

    public static UiMappedMessage Map(string title, string detail, bool isSuccess)
    {
        var normalized = detail?.Trim() ?? string.Empty;
        if (isSuccess)
        {
            return new UiMappedMessage(title, normalized, string.Empty, string.Empty, string.Empty, string.Empty, string.Empty);
        }

        var techFields = string.Join(", ", TechTokenRegex.Matches(normalized).Select(m => m.Value).Distinct(StringComparer.OrdinalIgnoreCase));
        var hasInternalField = normalized.Contains("single_fallback", StringComparison.OrdinalIgnoreCase)
            || normalized.Contains("UNVERIFIED", StringComparison.OrdinalIgnoreCase)
            || normalized.Contains("invalid_hmac", StringComparison.OrdinalIgnoreCase)
            || normalized.Contains("status=", StringComparison.OrdinalIgnoreCase)
            || normalized.Contains("route=", StringComparison.OrdinalIgnoreCase);

        if (hasInternalField)
        {
            return new UiMappedMessage(
                title,
                AppStrings.Get("ui.error.processing_failed"),
                AppStrings.Get("ui.error.next.open_diagnostics"),
                "diag.internal_state",
                normalized,
                normalized,
                techFields);
        }

        if (normalized.Contains("No such file", StringComparison.OrdinalIgnoreCase)
            || normalized.Contains("not found", StringComparison.OrdinalIgnoreCase)
            || normalized.Contains("path", StringComparison.OrdinalIgnoreCase))
        {
            return new UiMappedMessage(
                title,
                AppStrings.Get("ui.error.processing_failed"),
                AppStrings.Get("ui.error.next.check_input_path"),
                "diag.path",
                normalized,
                normalized,
                techFields);
        }

        return new UiMappedMessage(
            title,
            AppStrings.Get("ui.error.processing_failed"),
            AppStrings.Get("ui.error.next.retry"),
            "diag.generic",
            normalized,
            normalized,
            techFields);
    }
}
