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
    public string SlotHintText => "当前版本嵌入仍写槽位 0，槽位切换将在后续协议生效阶段接入。";
    public KeySlotSummary? ActiveKeySummary => _allSlotSummaries.FirstOrDefault(item => item.IsActive);
    public string ActiveSummaryTitle => $"槽位 {ActiveKeySlot}（{(ActiveKeySummary?.HasKey == true ? "已配置" : "未配置")}）";
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

    public Brush GenerateActionBrush => IsGenerateSuccess ? SuccessBrush : ResolvePrimaryTextBrush();
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

    public async Task GenerateKeyAsync()
    {
        if (IsBusy)
        {
            return;
        }

        IsBusy = true;
        try
        {
            var (_, error) = await Task.Run(() => AwmKeyBridge.GenerateAndSaveKeyInSlot(SelectedSlot));
            if (error == AwmError.Ok)
            {
                await _appViewModel.RefreshRuntimeStatusAsync();
                await RefreshSlotSummariesAsync();
                await FlashGenerateSuccessAsync();
            }
        }
        finally
        {
            IsBusy = false;
            RaiseComputedProperties();
        }
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
        OnPropertyChanged(nameof(ActiveKeySummary));
        OnPropertyChanged(nameof(ActiveSummaryTitle));
        OnPropertyChanged(nameof(ActiveSummaryKeyLine));
        OnPropertyChanged(nameof(ActiveSummaryEvidenceLine));
        OnPropertyChanged(nameof(ConfiguredSlotCount));
        OnPropertyChanged(nameof(ShowConfiguredSlotCount));
        OnPropertyChanged(nameof(ConfiguredSlotCountText));
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

    private async Task FlashGenerateSuccessAsync()
    {
        IsGenerateSuccess = true;
        await Task.Delay(1000);
        IsGenerateSuccess = false;
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
}
