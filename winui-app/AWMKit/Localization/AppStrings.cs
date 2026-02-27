using Microsoft.Windows.ApplicationModel.Resources;
using System;
using System.Globalization;
using Windows.Globalization;

namespace AWMKit.Localization;

public static class AppStrings
{
    private static readonly ResourceManager ResourceManager = new();
    private static readonly ResourceMap ResourceMap = ResourceManager.MainResourceMap.GetSubtree("Resources");

    public static string Get(string key, params object[] args)
    {
        var value = ResolveValue(key);
        if (args.Length == 0)
        {
            return value;
        }

        return string.Format(CultureInfo.CurrentCulture, value, args);
    }

    // Transitional helper to centralize language selection while migrating hardcoded pairs to keys.
    public static string Pick(string zh, string en)
    {
        return string.Equals(CurrentLanguageCode(), "en-US", StringComparison.OrdinalIgnoreCase) ? en : zh;
    }

    private static string ResolveValue(string key)
    {
        var lang = NormalizeLanguageCode(CurrentLanguageCode());
        return TryGet(key, lang)
            ?? TryGet(key, "en-US")
            ?? TryGet(key, "zh-CN")
            ?? key;
    }

    private static string? TryGet(string key, string language)
    {
        var context = ResourceManager.CreateResourceContext();
        context.QualifierValues["Language"] = language;
        var candidate = ResourceMap.TryGetValue(key, context);
        if (candidate is null)
        {
            return null;
        }

        var value = candidate.ValueAsString;
        return string.IsNullOrWhiteSpace(value) ? null : value;
    }

    private static string NormalizeLanguageCode(string? value)
    {
        return string.Equals(value?.Trim(), "en-US", StringComparison.OrdinalIgnoreCase)
            ? "en-US"
            : "zh-CN";
    }

    private static string CurrentLanguageCode()
    {
        if (!string.IsNullOrWhiteSpace(ApplicationLanguages.PrimaryLanguageOverride))
        {
            return ApplicationLanguages.PrimaryLanguageOverride;
        }

        return CultureInfo.CurrentUICulture.Name;
    }
}
