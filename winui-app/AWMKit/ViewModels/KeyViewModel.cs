using CommunityToolkit.Mvvm.ComponentModel;
using AWMKit.Models;
using AWMKit.Native;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
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
    public InfoBarSeverity KeyStatusSeverity => KeyAvailable ? InfoBarSeverity.Success : InfoBarSeverity.Warning;
    public string KeyStatusMessage => KeyAvailable
        ? AppStrings.Pick("密钥已配置，可正常嵌入与检测。", "Key is configured and ready for embed/detect.")
        : AppStrings.Pick("未配置密钥。请先生成密钥后再执行嵌入/检测。", "No key configured. Generate a key before embed/detect.");
    public bool SelectedSlotHasKey
    {
        get
        {
            var summary = SlotSummaries.FirstOrDefault(item => item.Slot == SelectedSlot);
            return summary?.HasKey ?? false;
        }
    }
    public string KeySourceLabel => _appViewModel.KeySourceLabel;
    public bool CanOperate => !IsBusy;
    public bool CanGenerateKey => !IsBusy && !SelectedSlotHasKey;
    public bool CanImportKey => !IsBusy && !SelectedSlotHasKey;
    public bool CanExportKey => !IsBusy && SelectedSlotHasKey;
    public string GenerateKeyTooltip => SelectedSlotHasKey
        ? AppStrings.Pick("当前槽位已有密钥，已禁止覆盖。请先删除后再生成。", "A key already exists in this slot. Delete it before generating.")
        : AppStrings.Pick("在当前槽位生成新密钥", "Generate a new key in current slot");
    public int ActiveKeySlot => _appViewModel.ActiveKeySlot;
    public string ActiveKeySlotText => AppStrings.Pick($"当前激活槽位：{ActiveKeySlot}", $"Active slot: {ActiveKeySlot}");
    public string KeyStatusText => KeyAvailable ? AppStrings.Pick("已配置", "Configured") : AppStrings.Pick("未配置", "Not configured");
    public Brush KeyStatusBrush => KeyAvailable ? ResolveSuccessBrush() : ResolveWarningBrush();
    public KeySlotSummary? ActiveKeySummary => _allSlotSummaries.FirstOrDefault(item => item.IsActive);
    public string ActiveSummaryTitle => AppStrings.Pick(
        $"槽位 {ActiveKeySlot}（{(ActiveKeySummary?.HasKey == true ? "已配置" : "未配置")}）",
        $"Slot {ActiveKeySlot} ({(ActiveKeySummary?.HasKey == true ? "configured" : "empty")})");
    public Brush ActiveSummaryTitleBrush => ResolveSuccessBrush();
    public string ActiveSummaryKeyLine
    {
        get
        {
            if (ActiveKeySummary is not { HasKey: true } summary)
            {
                return AppStrings.Pick("未配置", "Not configured");
            }

            return string.IsNullOrWhiteSpace(summary.Label)
                ? $"Key ID: {summary.KeyId ?? "-"}"
                : $"Key ID: {summary.KeyId ?? "-"} · {summary.Label}";
        }
    }
    public string ActiveSummaryEvidenceLine => AppStrings.Pick($"证据: {ActiveKeySummary?.EvidenceCount ?? 0}", $"Evidence: {ActiveKeySummary?.EvidenceCount ?? 0}");
    public int ConfiguredSlotCount => _allSlotSummaries.Count(item => item.HasKey);
    public bool ShowConfiguredSlotCount => ConfiguredSlotCount > 0;
    public string ConfiguredSlotCountText => ConfiguredSlotCount.ToString(System.Globalization.CultureInfo.InvariantCulture);
    public string KeyPageTitle => AppStrings.Pick("密钥管理", "Key management");
    public string KeyStatusFieldLabel => AppStrings.Pick("密钥状态", "Key status");
    public string KeySourceFieldLabel => AppStrings.Pick("密钥来源", "Key source");
    public string ActiveSlotFieldLabel => AppStrings.Pick("激活槽位", "Active slot");
    public string ApplyActionText => AppStrings.Pick("应用", "Apply");
    public string ActiveKeySectionTitle => AppStrings.Pick("当前激活密钥", "Current active key");
    public string GenerateActionText => AppStrings.Pick("生成", "Generate");
    public string ImportFileActionText => AppStrings.Pick("导入(.bin)", "Import (.bin)");
    public string ImportHexActionText => AppStrings.Pick("Hex 导入", "Hex Import");
    public string ExportFileActionText => AppStrings.Pick("导出(.bin)", "Export (.bin)");
    public string EditActionText => AppStrings.Pick("编辑", "Edit");
    public string DeleteActionText => AppStrings.Pick("删除", "Delete");
    public string RefreshActionText => AppStrings.Pick("刷新", "Refresh");
    public string SlotSummaryTitle => AppStrings.Pick("槽位摘要", "Slot summary");
    public string SlotSearchPlaceholder => AppStrings.Pick("搜索槽位 / Key ID / 标签 / 状态", "Search slot / Key ID / label / status");

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

    public Brush ApplyActionBrush => IsApplySuccess ? ResolveSuccessBrush() : ResolvePrimaryTextBrush();
    public Brush GenerateActionBrush => IsGenerateSuccess ? ResolveSuccessBrush() : ResolveAccentTextBrush();
    public Brush ImportActionBrush => IsImportSuccess ? ResolveSuccessBrush() : ResolvePrimaryTextBrush();
    public Brush ImportHexActionBrush => IsHexImportSuccess ? ResolveSuccessBrush() : ResolvePrimaryTextBrush();
    public Brush ExportActionBrush => IsExportSuccess ? ResolveSuccessBrush() : ResolvePrimaryTextBrush();
    public Brush EditActionBrush => IsEditSuccess ? ResolveSuccessBrush() : ResolvePrimaryTextBrush();
    public Brush DeleteActionBrush => IsDeleteSuccess ? ResolveSuccessBrush() : ResolvePrimaryTextBrush();
    public Brush RefreshActionBrush => IsRefreshSuccess ? ResolveSuccessBrush() : ResolvePrimaryTextBrush();

    private bool _isImportSuccess;
    public bool IsImportSuccess
    {
        get => _isImportSuccess;
        private set
        {
            if (SetProperty(ref _isImportSuccess, value))
            {
                OnPropertyChanged(nameof(ImportActionBrush));
            }
        }
    }

    private bool _isHexImportSuccess;
    public bool IsHexImportSuccess
    {
        get => _isHexImportSuccess;
        private set
        {
            if (SetProperty(ref _isHexImportSuccess, value))
            {
                OnPropertyChanged(nameof(ImportHexActionBrush));
            }
        }
    }

    private bool _isExportSuccess;
    public bool IsExportSuccess
    {
        get => _isExportSuccess;
        private set
        {
            if (SetProperty(ref _isExportSuccess, value))
            {
                OnPropertyChanged(nameof(ExportActionBrush));
            }
        }
    }

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

    public async Task<AwmError> ImportKeyBytesAsync(byte[] key)
    {
        return await ImportKeyBytesInternalAsync(key, isHexImport: false);
    }

    public async Task<AwmError> ImportHexAsync(string? hexInput)
    {
        var normalized = NormalizeHexKey(hexInput);
        if (normalized is null)
        {
            return AwmError.InvalidMessageLength;
        }

        var key = Convert.FromHexString(normalized);
        return await ImportKeyBytesInternalAsync(key, isHexImport: true);
    }

    public async Task<(byte[]? key, AwmError error)> ExportKeyBytesAsync()
    {
        if (IsBusy)
        {
            return (null, AwmError.AudiowmarkExec);
        }

        if (!SelectedSlotHasKey)
        {
            return (null, AwmError.AudiowmarkExec);
        }

        IsBusy = true;
        try
        {
            return await Task.Run(() => AwmKeyBridge.LoadKeyInSlot(SelectedSlot));
        }
        finally
        {
            IsBusy = false;
            RaiseComputedProperties();
        }
    }

    public async Task MarkExportSuccessAsync()
    {
        await FlashExportSuccessAsync();
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
            case nameof(AppViewModel.UiLanguageCode):
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
        OnPropertyChanged(nameof(KeyStatusSeverity));
        OnPropertyChanged(nameof(KeyStatusMessage));
        OnPropertyChanged(nameof(ActiveKeySummary));
        OnPropertyChanged(nameof(ActiveSummaryTitle));
        OnPropertyChanged(nameof(ActiveSummaryTitleBrush));
        OnPropertyChanged(nameof(ActiveSummaryKeyLine));
        OnPropertyChanged(nameof(ActiveSummaryEvidenceLine));
        OnPropertyChanged(nameof(ConfiguredSlotCount));
        OnPropertyChanged(nameof(ShowConfiguredSlotCount));
        OnPropertyChanged(nameof(ConfiguredSlotCountText));
        OnPropertyChanged(nameof(CanOperate));
        OnPropertyChanged(nameof(CanGenerateKey));
        OnPropertyChanged(nameof(CanImportKey));
        OnPropertyChanged(nameof(CanExportKey));
        OnPropertyChanged(nameof(GenerateKeyTooltip));
        OnPropertyChanged(nameof(KeyPageTitle));
        OnPropertyChanged(nameof(KeyStatusFieldLabel));
        OnPropertyChanged(nameof(KeySourceFieldLabel));
        OnPropertyChanged(nameof(ActiveSlotFieldLabel));
        OnPropertyChanged(nameof(ApplyActionText));
        OnPropertyChanged(nameof(ActiveKeySectionTitle));
        OnPropertyChanged(nameof(GenerateActionText));
        OnPropertyChanged(nameof(ImportFileActionText));
        OnPropertyChanged(nameof(ImportHexActionText));
        OnPropertyChanged(nameof(ExportFileActionText));
        OnPropertyChanged(nameof(EditActionText));
        OnPropertyChanged(nameof(DeleteActionText));
        OnPropertyChanged(nameof(RefreshActionText));
        OnPropertyChanged(nameof(SlotSummaryTitle));
        OnPropertyChanged(nameof(SlotSearchPlaceholder));
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
        var query = string.IsNullOrWhiteSpace(keyword) ? null : keyword;
        var filtered = query is null
            ? _allSlotSummaries
            : _allSlotSummaries.Where(item =>
            {
                var fields = string.Join(" ", new[]
                {
                    $"slot {item.Slot}",
                    $"槽位 {item.Slot}",
                    item.KeyId ?? string.Empty,
                    item.Label ?? string.Empty,
                    item.StatusText,
                    $"evidence {item.EvidenceCount}",
                    $"证据 {item.EvidenceCount}"
                });
                return fields.Contains(query, StringComparison.OrdinalIgnoreCase);
            }).ToList();

        FilteredSlotSummaries.Clear();
        foreach (var row in filtered)
        {
            FilteredSlotSummaries.Add(row);
        }
    }

    private static Brush ResolvePrimaryTextBrush()
    {
        return ResolveBrush("TextFillColorPrimaryBrush", "NeutralBrush");
    }

    private static Brush ResolveAccentTextBrush()
    {
        return ResolveBrush("TextOnAccentFillColorPrimaryBrush", "TextFillColorPrimaryBrush");
    }

    private static Brush ResolveSuccessBrush()
    {
        return ResolveBrush("SuccessBrush", "TextFillColorPrimaryBrush");
    }

    private static Brush ResolveWarningBrush()
    {
        return ResolveBrush("WarningBrush", "TextFillColorSecondaryBrush");
    }

    private static Brush ResolveBrush(string key, string fallbackKey)
    {
        if (Application.Current.Resources.TryGetValue(key, out var value)
            && value is Brush brush)
        {
            return brush;
        }

        if (Application.Current.Resources.TryGetValue(fallbackKey, out var fallbackValue)
            && fallbackValue is Brush fallbackBrush)
        {
            return fallbackBrush;
        }

        return new SolidColorBrush(Microsoft.UI.Colors.Transparent);
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

    private async Task FlashImportSuccessAsync()
    {
        IsImportSuccess = true;
        await Task.Delay(1000);
        IsImportSuccess = false;
    }

    private async Task FlashHexImportSuccessAsync()
    {
        IsHexImportSuccess = true;
        await Task.Delay(1000);
        IsHexImportSuccess = false;
    }

    private async Task FlashExportSuccessAsync()
    {
        IsExportSuccess = true;
        await Task.Delay(1000);
        IsExportSuccess = false;
    }

    private async Task<AwmError> ImportKeyBytesInternalAsync(byte[] key, bool isHexImport)
    {
        if (IsBusy)
        {
            return AwmError.AudiowmarkExec;
        }

        if (SelectedSlotHasKey)
        {
            return AwmError.KeyAlreadyExists;
        }

        if (key.Length != 32)
        {
            return AwmError.InvalidMessageLength;
        }

        IsBusy = true;
        try
        {
            var error = await Task.Run(() => AwmKeyBridge.SaveKeyInSlot(SelectedSlot, key));
            if (error == AwmError.Ok)
            {
                await _appViewModel.RefreshRuntimeStatusAsync();
                await RefreshSlotSummariesAsync();
                if (isHexImport)
                {
                    await FlashHexImportSuccessAsync();
                }
                else
                {
                    await FlashImportSuccessAsync();
                }
            }
            else
            {
                await RefreshSlotSummariesAsync();
            }
            return error;
        }
        finally
        {
            IsBusy = false;
            RaiseComputedProperties();
        }
    }

    private static string? NormalizeHexKey(string? input)
    {
        if (string.IsNullOrWhiteSpace(input))
        {
            return null;
        }

        var compact = string.Concat(input.Where(ch => !char.IsWhiteSpace(ch)));
        if (compact.StartsWith("0x", StringComparison.OrdinalIgnoreCase))
        {
            compact = compact[2..];
        }

        if (compact.Length != 64)
        {
            return null;
        }

        return compact.All(IsHexChar) ? compact : null;
    }

    private static bool IsHexChar(char ch)
    {
        return (ch >= '0' && ch <= '9')
               || (ch >= 'a' && ch <= 'f')
               || (ch >= 'A' && ch <= 'F');
    }


}
