using CommunityToolkit.Mvvm.ComponentModel;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Media;
using System;

namespace AWMKit.Models;

public enum LogIconTone
{
    Success,
    Info,
    Warning,
    Error,
}

public enum LogKind
{
    Generic,
    QueueCleared,
    LogsCleared,
    ResultOk,
    ResultNotFound,
    ResultInvalidHmac,
    ResultError,
}

/// <summary>
/// UI log entry for detect page timeline.
/// </summary>
public sealed partial class LogEntry : ObservableObject
{
    private bool _isSelected;
    public bool IsSelected
    {
        get => _isSelected;
        set
        {
            if (SetProperty(ref _isSelected, value))
            {
                OnPropertyChanged(nameof(CardBorderBrush));
                OnPropertyChanged(nameof(CardBackgroundBrush));
            }
        }
    }

    public Guid Id { get; init; } = Guid.NewGuid();
    public required string Title { get; init; }
    public string Detail { get; init; } = string.Empty;
    public string UserReason { get; init; } = string.Empty;
    public string NextAction { get; init; } = string.Empty;
    public string DiagnosticCode { get; init; } = string.Empty;
    public string DiagnosticDetail { get; init; } = string.Empty;
    public string RawError { get; init; } = string.Empty;
    public string TechFields { get; init; } = string.Empty;
    public bool IsSuccess { get; init; }
    public bool IsEphemeral { get; init; }
    public Guid? RelatedRecordId { get; init; }
    public LogKind Kind { get; init; } = LogKind.Generic;
    public DateTime Timestamp { get; init; } = DateTime.Now;
    public LogIconTone IconTone { get; init; } = LogIconTone.Info;

    public bool IsSelectable => RelatedRecordId.HasValue;
    public bool HasDiagnosticDetail => !string.IsNullOrWhiteSpace(DiagnosticDetail);

    private LogIconTone EffectiveIconTone => Kind switch
    {
        LogKind.QueueCleared => LogIconTone.Success,
        LogKind.LogsCleared => LogIconTone.Success,
        LogKind.ResultOk => LogIconTone.Success,
        LogKind.ResultNotFound => LogIconTone.Warning,
        LogKind.ResultInvalidHmac => LogIconTone.Error,
        LogKind.ResultError => LogIconTone.Error,
        _ => IconTone,
    };

    public string IconGlyph => EffectiveIconTone switch
    {
        LogIconTone.Success => "\uE73E",
        LogIconTone.Warning => "\uE7BA",
        LogIconTone.Error => "\uEA39",
        _ => "\uE946",
    };

    public Brush IconBrush => EffectiveIconTone switch
    {
        LogIconTone.Success => ResolveBrush("SuccessBrush", "TextFillColorSecondaryBrush"),
        LogIconTone.Warning => ResolveBrush("WarningBrush", "TextFillColorSecondaryBrush"),
        LogIconTone.Error => ResolveBrush("ErrorBrush", "TextFillColorSecondaryBrush"),
        _ => ResolveBrush("InfoBrush", "TextFillColorSecondaryBrush"),
    };

    public Brush CardBorderBrush => IsSelected
        ? ResolveBrush("SelectionBorderBrush", "AccentFillColorDefaultBrush")
        : ResolveBrush("TransparentBrush", "SubtleFillColorTransparentBrush");

    public Brush CardBackgroundBrush => IsSelected
        ? ResolveBrush("SelectionBackgroundBrush", "AccentFillColorSecondaryBrush")
        : ResolveBrush("TransparentBrush", "SubtleFillColorTransparentBrush");

    private static Brush ResolveBrush(string key, string fallbackKey)
    {
        var resources = Application.Current.Resources;
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
