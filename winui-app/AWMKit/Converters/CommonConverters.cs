using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Data;
using Microsoft.UI.Xaml.Media;
using System;

namespace AWMKit.Converters;

/// <summary>
/// Converts null to false for IsOpen binding.
/// </summary>
public sealed partial class NullToFalseConverter : IValueConverter
{
    public object Convert(object value, Type targetType, object parameter, string language)
    {
        return value is not null;
    }

    public object ConvertBack(object value, Type targetType, object parameter, string language)
    {
        throw new NotImplementedException();
    }
}

/// <summary>
/// Converts null to Collapsed for Visibility binding.
/// </summary>
public sealed partial class NullToCollapsedConverter : IValueConverter
{
    public object Convert(object value, Type targetType, object parameter, string language)
    {
        return value is null ? Visibility.Collapsed : Visibility.Visible;
    }

    public object ConvertBack(object value, Type targetType, object parameter, string language)
    {
        throw new NotImplementedException();
    }
}

/// <summary>
/// Inverts boolean value.
/// </summary>
public sealed partial class InverseBoolConverter : IValueConverter
{
    public object Convert(object value, Type targetType, object parameter, string language)
    {
        return value is bool b && !b;
    }

    public object ConvertBack(object value, Type targetType, object parameter, string language)
    {
        return value is bool b && !b;
    }
}

/// <summary>
/// Converts inverse bool to Visibility.
/// </summary>
public sealed partial class InverseBoolToVisibilityConverter : IValueConverter
{
    public object Convert(object value, Type targetType, object parameter, string language)
    {
        return value is bool b && b ? Visibility.Collapsed : Visibility.Visible;
    }

    public object ConvertBack(object value, Type targetType, object parameter, string language)
    {
        throw new NotImplementedException();
    }
}

/// <summary>
/// Converts bool to Visibility.
/// </summary>
public sealed partial class BoolToVisibilityConverter : IValueConverter
{
    public object Convert(object value, Type targetType, object parameter, string language)
    {
        return value is bool b && b ? Visibility.Visible : Visibility.Collapsed;
    }

    public object ConvertBack(object value, Type targetType, object parameter, string language)
    {
        throw new NotImplementedException();
    }
}

/// <summary>
/// Converts count to Visibility (visible if count > 0).
/// </summary>
public sealed partial class CountToVisibilityConverter : IValueConverter
{
    public object Convert(object value, Type targetType, object parameter, string language)
    {
        return value is int count && count > 0 ? Visibility.Visible : Visibility.Collapsed;
    }

    public object ConvertBack(object value, Type targetType, object parameter, string language)
    {
        throw new NotImplementedException();
    }
}

/// <summary>
/// Converts zero to Visibility (visible if count == 0).
/// </summary>
public sealed partial class ZeroToVisibilityConverter : IValueConverter
{
    public object Convert(object value, Type targetType, object parameter, string language)
    {
        return value is int count && count == 0 ? Visibility.Visible : Visibility.Collapsed;
    }

    public object ConvertBack(object value, Type targetType, object parameter, string language)
    {
        throw new NotImplementedException();
    }
}

/// <summary>
/// Converts DateTime to readable string.
/// </summary>
public sealed partial class DateTimeConverter : IValueConverter
{
    public object Convert(object value, Type targetType, object parameter, string language)
    {
        if (value is DateTime dt)
        {
            return dt.ToLocalTime().ToString("yyyy-MM-dd HH:mm:ss", System.Globalization.CultureInfo.InvariantCulture);
        }
        return string.Empty;
    }

    public object ConvertBack(object value, Type targetType, object parameter, string language)
    {
        throw new NotImplementedException();
    }
}

/// <summary>
/// Formats DateTime into 3 centered Chinese lines:
/// line1: 年, line2: 月日, line3: 时分秒.
/// </summary>
public sealed partial class ChineseThreeLineDateTimeConverter : IValueConverter
{
    public object Convert(object value, Type targetType, object parameter, string language)
    {
        DateTime? dt = value switch
        {
            DateTime dateTime => dateTime,
            DateTimeOffset dateTimeOffset => dateTimeOffset.DateTime,
            _ => null
        };

        if (dt is not DateTime raw)
        {
            return string.Empty;
        }

        var local = raw.ToLocalTime();
        return $"{local:yyyy年}\n{local:M月d日}\n{local:HH:mm:ss}";
    }

    public object ConvertBack(object value, Type targetType, object parameter, string language)
    {
        throw new NotImplementedException();
    }
}

/// <summary>
/// Formats display name with parentheses (null-safe).
/// </summary>
public sealed partial class DisplayNameConverter : IValueConverter
{
    public object Convert(object value, Type targetType, object parameter, string language)
    {
        return value is string name && !string.IsNullOrEmpty(name) ? $"({name})" : string.Empty;
    }

    public object ConvertBack(object value, Type targetType, object parameter, string language)
    {
        throw new NotImplementedException();
    }
}

/// <summary>
/// Converts bool success to color brush (green/red).
/// </summary>
public sealed partial class BoolToSuccessColorConverter : IValueConverter
{
    public object Convert(object value, Type targetType, object parameter, string language)
    {
        if (value is bool success)
        {
            return success
                ? App.Current.Resources["SuccessBrush"]
                : App.Current.Resources["ErrorBrush"];
        }
        return App.Current.Resources["NeutralBrush"];
    }

    public object ConvertBack(object value, Type targetType, object parameter, string language)
    {
        throw new NotImplementedException();
    }
}

/// <summary>
/// Converts bool success to detection text.
/// </summary>
public sealed partial class BoolToDetectionTextConverter : IValueConverter
{
    public object Convert(object value, Type targetType, object parameter, string language)
    {
        return value is bool success && success ? "Detected" : "Not Detected";
    }

    public object ConvertBack(object value, Type targetType, object parameter, string language)
    {
        throw new NotImplementedException();
    }
}

/// <summary>
/// Converts selected flag to row border brush.
/// </summary>
public sealed partial class BoolToSelectionBorderBrushConverter : IValueConverter
{
    public object Convert(object value, Type targetType, object parameter, string language)
    {
        var selected = value is bool isSelected && isSelected;
        var resources = Application.Current.Resources;
        return selected
            ? ResolveBrush(resources, "SelectionBorderBrush", "AccentFillColorDefaultBrush")
            : ResolveBrush(resources, "CardStrokeColorDefaultBrush", "TextFillColorSecondaryBrush");
    }

    public object ConvertBack(object value, Type targetType, object parameter, string language)
    {
        throw new NotImplementedException();
    }

    private static Brush ResolveBrush(ResourceDictionary resources, string key, string fallbackKey)
    {
        if (resources.TryGetValue(key, out var value) && value is Brush brush)
        {
            return brush;
        }

        if (resources.TryGetValue(fallbackKey, out var fallbackValue) && fallbackValue is Brush fallbackBrush)
        {
            return fallbackBrush;
        }

        return new SolidColorBrush(Microsoft.UI.Colors.Transparent);
    }
}

/// <summary>
/// Converts selected flag to row background brush.
/// </summary>
public sealed partial class BoolToSelectionBackgroundBrushConverter : IValueConverter
{
    public object Convert(object value, Type targetType, object parameter, string language)
    {
        var selected = value is bool isSelected && isSelected;
        var resources = Application.Current.Resources;
        return selected
            ? ResolveBrush(resources, "SelectionBackgroundBrush", "AccentFillColorSecondaryBrush")
            : ResolveBrush(resources, "TransparentBrush", "SubtleFillColorTransparentBrush");
    }

    public object ConvertBack(object value, Type targetType, object parameter, string language)
    {
        throw new NotImplementedException();
    }

    private static Brush ResolveBrush(ResourceDictionary resources, string key, string fallbackKey)
    {
        if (resources.TryGetValue(key, out var value) && value is Brush brush)
        {
            return brush;
        }

        if (resources.TryGetValue(fallbackKey, out var fallbackValue) && fallbackValue is Brush fallbackBrush)
        {
            return fallbackBrush;
        }

        return new SolidColorBrush(Microsoft.UI.Colors.Transparent);
    }
}

/// <summary>
/// Converts key slot status text to semantic brush.
/// </summary>
public sealed partial class KeySlotStatusBrushConverter : IValueConverter
{
    public object Convert(object value, Type targetType, object parameter, string language)
    {
        var resources = Application.Current.Resources;
        var status = value as string ?? string.Empty;
        return status switch
        {
            "active" => ResolveBrush(resources, "SuccessBrush"),
            "duplicate" => ResolveBrush(resources, "WarningBrush"),
            "configured" => ResolveBrush(resources, "TextFillColorPrimaryBrush"),
            _ => ResolveBrush(resources, "TextFillColorSecondaryBrush")
        };
    }

    public object ConvertBack(object value, Type targetType, object parameter, string language)
    {
        throw new NotImplementedException();
    }

    private static Brush ResolveBrush(ResourceDictionary resources, string key)
    {
        if (resources.TryGetValue(key, out var value) && value is Brush brush)
        {
            return brush;
        }
        return ResolveBrush(resources, "TextFillColorSecondaryBrush", "NeutralBrush");
    }

    private static Brush ResolveBrush(ResourceDictionary resources, string key, string fallbackKey)
    {
        if (resources.TryGetValue(key, out var value) && value is Brush brush)
        {
            return brush;
        }

        if (resources.TryGetValue(fallbackKey, out var fallbackValue) && fallbackValue is Brush fallbackBrush)
        {
            return fallbackBrush;
        }

        return new SolidColorBrush(Microsoft.UI.Colors.Transparent);
    }
}

/// <summary>
/// Converts slot active flag to slot icon brush (active=green, inactive=secondary).
/// </summary>
public sealed partial class BoolToKeySlotIconBrushConverter : IValueConverter
{
    public object Convert(object value, Type targetType, object parameter, string language)
    {
        var resources = Application.Current.Resources;
        var isActive = value is bool active && active;

        if (isActive)
        {
            return ResolveBrush(resources, "SuccessBrush");
        }

        return ResolveBrush(resources, "TextFillColorSecondaryBrush");
    }

    public object ConvertBack(object value, Type targetType, object parameter, string language)
    {
        throw new NotImplementedException();
    }

    private static Brush ResolveBrush(ResourceDictionary resources, string key)
    {
        if (resources.TryGetValue(key, out var value) && value is Brush brush)
        {
            return brush;
        }
        if (resources.TryGetValue("TextFillColorSecondaryBrush", out var fallback) && fallback is Brush fallbackBrush)
        {
            return fallbackBrush;
        }

        return new SolidColorBrush(Microsoft.UI.Colors.Transparent);
    }
}
