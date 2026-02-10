using CommunityToolkit.Mvvm.ComponentModel;
using AWMKit.Models;
using AWMKit.Native;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Media;
using System;
using System.Collections.Generic;
using System.Collections.ObjectModel;
using System.ComponentModel;
using System.Linq;
using System.Threading.Tasks;

namespace AWMKit.ViewModels;

/// <summary>
/// Key management page state model.
/// </summary>
public sealed partial class KeyViewModel : ObservableObject
{
    private readonly AppViewModel _appViewModel = AppViewModel.Instance;
    private readonly List<KeySlotSummary> _allSlotSummaries = new();
    private static readonly SolidColorBrush SuccessBrush = new(Windows.UI.Color.FromArgb(255, 76, 175, 80));

    private bool _isBusy;
    public bool IsBusy
    {
        get => _isBusy;
        private set => SetProperty(ref _isBusy, value);
    }

    private int _selectedSlot;
    public int SelectedSlot
    {
        get => _selectedSlot;
        set
        {
            if (SetProperty(ref _selectedSlot, Math.Clamp(value, 0, 31)))
            {
                OnPropertyChanged(nameof(SelectedSlotHasKey));
            }
        }
    }

    public ObservableCollection<int> SlotOptions { get; } = new();
    public ObservableCollection<KeySlotSummary> SlotSummaries { get; } = new();
    public ObservableCollection<KeySlotSummary> FilteredSlotSummaries { get; } = new();

    private string _slotSearchText = string.Empty;
    public string SlotSearchText
    {
        get => _slotSearchText;
        set
        {
            if (SetProperty(ref _slotSearchText, value))
            {
                ApplySlotFilter();
            }
        }
    }

    public bool KeyAvailable => _appViewModel.KeyAvailable;
    public bool SelectedSlotHasKey
    {
        get
        {
            var summary = SlotSummaries.FirstOrDefault(item => item.Slot == SelectedSlot);
            return summary?.HasKey ?? false;
        }
    }
    public string KeySourceLabel => _appViewModel.KeySourceLabel;
    public int ActiveKeySlot => _appViewModel.ActiveKeySlot;
    public string ActiveKeySlotText => $"当前激活槽位：{ActiveKeySlot}";
    public string KeyStatusText => KeyAvailable ? "已配置" : "未配置";
    public Brush KeyStatusBrush => KeyAvailable ? SuccessBrush : ResolveWarningBrush();
    public KeySlotSummary? ActiveKeySummary => _allSlotSummaries.FirstOrDefault(item => item.IsActive);
    public string ActiveSummaryTitle => $"槽位 {ActiveKeySlot}（{(ActiveKeySummary?.HasKey == true ? "已配置" : "未配置")}）";
    public Brush ActiveSummaryTitleBrush => SuccessBrush;
    public string ActiveSummaryKeyLine
    {
        get
        {
            if (ActiveKeySummary is not { HasKey: true } summary)
            {
                return "未配置";
            }

            return string.IsNullOrWhiteSpace(summary.Label)
                ? $"Key ID: {summary.KeyId ?? "-"}"
                : $"Key ID: {summary.KeyId ?? "-"} · {summary.Label}";
        }
    }
    public string ActiveSummaryEvidenceLine => $"证据: {ActiveKeySummary?.EvidenceCount ?? 0}";
    public int ConfiguredSlotCount => _allSlotSummaries.Count(item => item.HasKey);
    public bool ShowConfiguredSlotCount => ConfiguredSlotCount > 0;
    public string ConfiguredSlotCountText => ConfiguredSlotCount.ToString();

    private bool _isGenerateSuccess;
    public bool IsGenerateSuccess
    {
        get => _isGenerateSuccess;
        private set
        {
            if (SetProperty(ref _isGenerateSuccess, value))
            {
                OnPropertyChanged(nameof(GenerateActionBrush));
            }
        }
    }

    private bool _isDeleteSuccess;
    public bool IsDeleteSuccess
    {
        get => _isDeleteSuccess;
        private set
        {
            if (SetProperty(ref _isDeleteSuccess, value))
            {
                OnPropertyChanged(nameof(DeleteActionBrush));
            }
        }
    }

    private bool _isRefreshSuccess;
    public bool IsRefreshSuccess
    {
        get => _isRefreshSuccess;
        private set
        {
            if (SetProperty(ref _isRefreshSuccess, value))
            {
                OnPropertyChanged(nameof(RefreshActionBrush));
            }
        }
    }

    private bool _isApplySuccess;
    public bool IsApplySuccess
    {
        get => _isApplySuccess;
        private set
        {
            if (SetProperty(ref _isApplySuccess, value))
            {
                OnPropertyChanged(nameof(ApplyActionBrush));
            }
        }
    }

    private bool _isEditSuccess;
    public bool IsEditSuccess
    {
        get => _isEditSuccess;
        private set
        {
            if (SetProperty(ref _isEditSuccess, value))
            {
                OnPropertyChanged(nameof(EditActionBrush));
            }
        }
    }

    public Brush ApplyActionBrush => IsApplySuccess ? SuccessBrush : ResolvePrimaryTextBrush();
    public Brush GenerateActionBrush => IsGenerateSuccess ? SuccessBrush : ResolveAccentTextBrush();
    public Brush EditActionBrush => IsEditSuccess ? SuccessBrush : ResolvePrimaryTextBrush();
    public Brush DeleteActionBrush => IsDeleteSuccess ? SuccessBrush : ResolvePrimaryTextBrush();
    public Brush RefreshActionBrush => IsRefreshSuccess ? SuccessBrush : ResolvePrimaryTextBrush();

    public KeyViewModel()
    {
        for (var i = 0; i <= 31; i++)
        {
            SlotOptions.Add(i);
        }

        _appViewModel.PropertyChanged += AppViewModelOnPropertyChanged;
    }

    public async Task InitializeAsync()
    {
        await _appViewModel.RefreshRuntimeStatusAsync();
        SelectedSlot = _appViewModel.ActiveKeySlot;
        await RefreshSlotSummariesAsync();
        RaiseComputedProperties();
    }

    public async Task<AwmError> GenerateKeyAsync(string? labelToApply = null)
    {
        if (IsBusy)
        {
            return AwmError.AudiowmarkExec;
        }

        IsBusy = true;
        var result = AwmError.AudiowmarkExec;
        try
        {
            var (_, error) = await Task.Run(() => AwmKeyBridge.GenerateAndSaveKeyInSlot(SelectedSlot));
            result = error;
            if (error == AwmError.Ok)
            {
                var trimmed = (labelToApply ?? string.Empty).Trim();
                if (!string.IsNullOrWhiteSpace(trimmed))
                {
                    var labelError = await Task.Run(() => AwmKeyBridge.SetSlotLabel(SelectedSlot, trimmed));
                    if (labelError != AwmError.Ok)
                    {
                        result = labelError;
                    }
                }
                await _appViewModel.RefreshRuntimeStatusAsync();
                await RefreshSlotSummariesAsync();
                await FlashGenerateSuccessAsync();
            }
            else
            {
                // Keep slot/status view in sync even when generation is rejected.
                await RefreshSlotSummariesAsync();
            }
        }
        finally
        {
            IsBusy = false;
            RaiseComputedProperties();
        }

        return result;
    }

    public async Task DeleteKeyAsync()
    {
        if (IsBusy)
        {
            return;
        }

        IsBusy = true;
        try
        {
            var (_, error) = await Task.Run(() => AwmKeyBridge.DeleteKeyInSlot(SelectedSlot));
            if (error == AwmError.Ok)
            {
                await _appViewModel.RefreshRuntimeStatusAsync();
                SelectedSlot = _appViewModel.ActiveKeySlot;
                await RefreshSlotSummariesAsync();
                await FlashDeleteSuccessAsync();
            }
        }
        finally
        {
            IsBusy = false;
            RaiseComputedProperties();
        }
    }

    public async Task SaveSelectedSlotAsync()
    {
        if (IsBusy)
        {
            return;
        }

        IsBusy = true;
        try
        {
            await _appViewModel.SetActiveKeySlotAsync(SelectedSlot);
            await _appViewModel.RefreshActiveKeySlotAsync();
            SelectedSlot = _appViewModel.ActiveKeySlot;
            await RefreshSlotSummariesAsync();
            await FlashApplySuccessAsync();
        }
        finally
        {
            IsBusy = false;
            RaiseComputedProperties();
        }
    }

    public async Task RefreshStatusAsync()
    {
        if (IsBusy)
        {
            return;
        }

        IsBusy = true;
        try
        {
            await _appViewModel.RefreshRuntimeStatusAsync();
            SelectedSlot = _appViewModel.ActiveKeySlot;
            await RefreshSlotSummariesAsync();
            await FlashRefreshSuccessAsync();
        }
        finally
        {
            IsBusy = false;
            RaiseComputedProperties();
        }
    }

    private void AppViewModelOnPropertyChanged(object? sender, PropertyChangedEventArgs e)
    {
        switch (e.PropertyName)
        {
            case nameof(AppViewModel.KeyAvailable):
            case nameof(AppViewModel.KeySourceLabel):
            case nameof(AppViewModel.ActiveKeySlot):
                RefreshSlotSummaries();
                if (e.PropertyName == nameof(AppViewModel.ActiveKeySlot))
                {
                    SelectedSlot = _appViewModel.ActiveKeySlot;
                }

                RaiseComputedProperties();
                break;
        }
    }

    private void RaiseComputedProperties()
    {
        OnPropertyChanged(nameof(KeyAvailable));
        OnPropertyChanged(nameof(SelectedSlotHasKey));
        OnPropertyChanged(nameof(KeySourceLabel));
        OnPropertyChanged(nameof(ActiveKeySlot));
        OnPropertyChanged(nameof(ActiveKeySlotText));
        OnPropertyChanged(nameof(KeyStatusText));
        OnPropertyChanged(nameof(KeyStatusBrush));
        OnPropertyChanged(nameof(ActiveKeySummary));
        OnPropertyChanged(nameof(ActiveSummaryTitle));
        OnPropertyChanged(nameof(ActiveSummaryTitleBrush));
        OnPropertyChanged(nameof(ActiveSummaryKeyLine));
        OnPropertyChanged(nameof(ActiveSummaryEvidenceLine));
        OnPropertyChanged(nameof(ConfiguredSlotCount));
        OnPropertyChanged(nameof(ShowConfiguredSlotCount));
        OnPropertyChanged(nameof(ConfiguredSlotCountText));
    }

    public async Task<AwmError> EditActiveSlotLabelAsync(string? label)
    {
        if (IsBusy)
        {
            return AwmError.AudiowmarkExec;
        }

        IsBusy = true;
        try
        {
            var activeSlot = _appViewModel.ActiveKeySlot;
            var trimmed = (label ?? string.Empty).Trim();
            var error = string.IsNullOrWhiteSpace(trimmed)
                ? await Task.Run(() => AwmKeyBridge.ClearSlotLabel(activeSlot))
                : await Task.Run(() => AwmKeyBridge.SetSlotLabel(activeSlot, trimmed));
            if (error == AwmError.Ok)
            {
                await _appViewModel.RefreshRuntimeStatusAsync();
                await RefreshSlotSummariesAsync();
                SelectedSlot = _appViewModel.ActiveKeySlot;
                await FlashEditSuccessAsync();
            }

            return error;
        }
        finally
        {
            IsBusy = false;
            RaiseComputedProperties();
        }
    }

    private Task RefreshSlotSummariesAsync()
    {
        RefreshSlotSummaries();
        return Task.CompletedTask;
    }

    private void RefreshSlotSummaries()
    {
        var (rows, error) = AwmKeyBridge.GetSlotSummaries();
        _allSlotSummaries.Clear();
        SlotSummaries.Clear();
        FilteredSlotSummaries.Clear();
        if (error == AwmError.Ok)
        {
            foreach (var row in rows.OrderBy(item => item.Slot))
            {
                _allSlotSummaries.Add(row);
                SlotSummaries.Add(row);
            }
        }

        ApplySlotFilter();
        OnPropertyChanged(nameof(SelectedSlotHasKey));
        OnPropertyChanged(nameof(ActiveKeySummary));
        OnPropertyChanged(nameof(ActiveSummaryTitle));
        OnPropertyChanged(nameof(ActiveSummaryKeyLine));
        OnPropertyChanged(nameof(ActiveSummaryEvidenceLine));
        OnPropertyChanged(nameof(ConfiguredSlotCount));
        OnPropertyChanged(nameof(ShowConfiguredSlotCount));
        OnPropertyChanged(nameof(ConfiguredSlotCountText));
    }

    private void ApplySlotFilter()
    {
        var keyword = SlotSearchText?.Trim();
        var query = string.IsNullOrWhiteSpace(keyword) ? null : keyword.ToLowerInvariant();
        var filtered = query is null
            ? _allSlotSummaries
            : _allSlotSummaries.Where(item =>
            {
                var fields = string.Join(" ", new[]
                {
                    $"槽位 {item.Slot}",
                    item.KeyId ?? string.Empty,
                    item.Label ?? string.Empty,
                    item.StatusText,
                    $"证据 {item.EvidenceCount}"
                }).ToLowerInvariant();
                return fields.Contains(query, StringComparison.Ordinal);
            }).ToList();

        FilteredSlotSummaries.Clear();
        foreach (var row in filtered)
        {
            FilteredSlotSummaries.Add(row);
        }
    }

    private static Brush ResolvePrimaryTextBrush()
    {
        if (Application.Current.Resources.TryGetValue("TextFillColorPrimaryBrush", out var value)
            && value is Brush brush)
        {
            return brush;
        }

        return new SolidColorBrush(Windows.UI.Color.FromArgb(255, 32, 32, 32));
    }

    private static Brush ResolveAccentTextBrush()
    {
        if (Application.Current.Resources.TryGetValue("TextOnAccentFillColorPrimaryBrush", out var value)
            && value is Brush brush)
        {
            return brush;
        }

        return new SolidColorBrush(Windows.UI.Color.FromArgb(255, 255, 255, 255));
    }

    private static Brush ResolveWarningBrush()
    {
        if (Application.Current.Resources.TryGetValue("WarningBrush", out var value)
            && value is Brush brush)
        {
            return brush;
        }

        return new SolidColorBrush(Windows.UI.Color.FromArgb(255, 255, 152, 0));
    }

    private async Task FlashGenerateSuccessAsync()
    {
        IsGenerateSuccess = true;
        await Task.Delay(1000);
        IsGenerateSuccess = false;
    }

    private async Task FlashApplySuccessAsync()
    {
        IsApplySuccess = true;
        await Task.Delay(1000);
        IsApplySuccess = false;
    }

    private async Task FlashDeleteSuccessAsync()
    {
        IsDeleteSuccess = true;
        await Task.Delay(1000);
        IsDeleteSuccess = false;
    }

    private async Task FlashRefreshSuccessAsync()
    {
        IsRefreshSuccess = true;
        await Task.Delay(1000);
        IsRefreshSuccess = false;
    }

    private async Task FlashEditSuccessAsync()
    {
        IsEditSuccess = true;
        await Task.Delay(1000);
        IsEditSuccess = false;
    }
}
