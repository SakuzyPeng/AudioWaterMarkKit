using CommunityToolkit.Mvvm.ComponentModel;
using Microsoft.UI;
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

/// <summary>
/// UI log entry for detect page timeline.
/// </summary>
public sealed partial class LogEntry : ObservableObject
{
    private static readonly SolidColorBrush SuccessBrush = new(Windows.UI.Color.FromArgb(255, 76, 175, 80));
    private static readonly SolidColorBrush InfoBrush = new(Windows.UI.Color.FromArgb(255, 33, 150, 243));
    private static readonly SolidColorBrush WarningBrush = new(Windows.UI.Color.FromArgb(255, 255, 152, 0));
    private static readonly SolidColorBrush ErrorBrush = new(Windows.UI.Color.FromArgb(255, 244, 67, 54));
    private static readonly SolidColorBrush NeutralBrush = new(Windows.UI.Color.FromArgb(0, 0, 0, 0));
    private static readonly SolidColorBrush AccentBorderBrush = new(Windows.UI.Color.FromArgb(255, 33, 150, 243));
    private static readonly SolidColorBrush AccentBackgroundBrush = new(Windows.UI.Color.FromArgb(30, 33, 150, 243));

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
    public bool IsSuccess { get; init; }
    public bool IsEphemeral { get; init; }
    public Guid? RelatedRecordId { get; init; }
    public DateTime Timestamp { get; init; } = DateTime.Now;
    public LogIconTone IconTone { get; init; } = LogIconTone.Info;

    public bool IsSelectable => RelatedRecordId.HasValue;

    public string IconGlyph => IconTone switch
    {
        LogIconTone.Success => "\uE73E",
        LogIconTone.Warning => "\uE7BA",
        LogIconTone.Error => "\uEA39",
        _ => "\uE946",
    };

    public Brush IconBrush => IconTone switch
    {
        LogIconTone.Success => SuccessBrush,
        LogIconTone.Warning => WarningBrush,
        LogIconTone.Error => ErrorBrush,
        _ => InfoBrush,
    };

    public Brush CardBorderBrush => IsSelected ? AccentBorderBrush : NeutralBrush;

    public Brush CardBackgroundBrush => IsSelected ? AccentBackgroundBrush : NeutralBrush;

}
