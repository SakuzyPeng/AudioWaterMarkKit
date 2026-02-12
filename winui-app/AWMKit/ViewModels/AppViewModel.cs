using AWMKit.Data;
using AWMKit.Models;
using AWMKit.Native;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Media;
using System;
using System.Collections.Generic;
using System.Globalization;
using System.Linq;
using System.Threading.Tasks;

namespace AWMKit.ViewModels;

/// <summary>
/// Application-level view model managing global state (key, engine, database).
/// Singleton instance shared across all pages.
/// </summary>
public sealed partial class AppViewModel : ObservableObject
{
    private static AppViewModel? _instance;
    public static AppViewModel Instance => _instance ??= new AppViewModel();

    private bool _keyAvailable;
    public bool KeyAvailable
    {
        get => _keyAvailable;
        set
        {
            if (SetProperty(ref _keyAvailable, value))
            {
                NotifyStatusPresentationChanged();
            }
        }
    }

    private string _keySourceLabel = "未配置";
    public string KeySourceLabel
    {
        get => _keySourceLabel;
        set
        {
            if (SetProperty(ref _keySourceLabel, value))
            {
                NotifyStatusPresentationChanged();
            }
        }
    }

    private bool _engineAvailable;
    public bool EngineAvailable
    {
        get => _engineAvailable;
        set
        {
            if (SetProperty(ref _engineAvailable, value))
            {
                NotifyStatusPresentationChanged();
            }
        }
    }

    private bool _databaseAvailable;
    public bool DatabaseAvailable
    {
        get => _databaseAvailable;
        set
        {
            if (SetProperty(ref _databaseAvailable, value))
            {
                NotifyStatusPresentationChanged();
            }
        }
    }

    private string _currentIdentity = string.Empty;
    public string CurrentIdentity
    {
        get => _currentIdentity;
        set => SetProperty(ref _currentIdentity, value);
    }

    private string _enginePath = string.Empty;
    public string EnginePath
    {
        get => _enginePath;
        set
        {
            if (SetProperty(ref _enginePath, value))
            {
                NotifyStatusPresentationChanged();
            }
        }
    }

    private string _engineBackend = "-";
    public string EngineBackend
    {
        get => _engineBackend;
        set
        {
            if (SetProperty(ref _engineBackend, value))
            {
                NotifyStatusPresentationChanged();
            }
        }
    }

    private string _engineContainers = "-";
    public string EngineContainers
    {
        get => _engineContainers;
        set
        {
            if (SetProperty(ref _engineContainers, value))
            {
                NotifyStatusPresentationChanged();
            }
        }
    }

    private string _engineEac3 = "unavailable";
    public string EngineEac3
    {
        get => _engineEac3;
        set
        {
            if (SetProperty(ref _engineEac3, value))
            {
                NotifyStatusPresentationChanged();
            }
        }
    }

    private int _totalTags;
    public int TotalTags
    {
        get => _totalTags;
        set
        {
            if (SetProperty(ref _totalTags, value))
            {
                NotifyStatusPresentationChanged();
            }
        }
    }

    private int _totalEvidence;
    public int TotalEvidence
    {
        get => _totalEvidence;
        set
        {
            if (SetProperty(ref _totalEvidence, value))
            {
                NotifyStatusPresentationChanged();
            }
        }
    }

    public Brush KeyStatusBrush => GetAvailabilityBrush(KeyAvailable);

    public Brush EngineStatusBrush => GetAvailabilityBrush(EngineAvailable);

    public Brush DatabaseStatusBrush => GetAvailabilityBrush(DatabaseAvailable);

    public string KeyStatusTooltip => BuildKeyStatusTooltip();

    public string EngineStatusTooltip => EngineAvailable
        ? $"{L("音频引擎", "Audio engine")}：{L("可用", "Available")}\n{L("路径", "Path")}：{EnginePath}\n{L("媒体后端", "Media backend")}：{EngineBackend}\nEAC3：{EngineEac3}\n{L("容器", "Containers")}：{EngineContainers}\n{L("点击刷新状态", "Click to refresh status")}"
        : $"{L("音频引擎", "Audio engine")}：{L("不可用", "Unavailable")}\n{L("请检查 bundled 或 PATH", "Check bundled binary or PATH")}\n{L("点击刷新状态", "Click to refresh status")}";

    public string DatabaseStatusTooltip => DatabaseAvailable
        ? $"{L("数据库", "Database")}：{L("可用", "Available")}\n{L("映射", "Mappings")}：{TotalTags}\n{L("证据", "Evidence")}：{TotalEvidence}\n{L("点击刷新状态", "Click to refresh status")}"
        : $"{L("数据库", "Database")}：{L("不可用", "Unavailable")}\n{L("点击刷新状态", "Click to refresh status")}";

    private string _uiLanguageCode = "zh-CN";
    public string UiLanguageCode
    {
        get => _uiLanguageCode;
        private set
        {
            if (SetProperty(ref _uiLanguageCode, value))
            {
                OnPropertyChanged(nameof(IsChineseLanguage));
                OnPropertyChanged(nameof(IsEnglishLanguage));
                OnPropertyChanged(nameof(ThemeSystemLabel));
                OnPropertyChanged(nameof(ThemeLightLabel));
                OnPropertyChanged(nameof(ThemeDarkLabel));
                OnPropertyChanged(nameof(LanguageChineseLabel));
                OnPropertyChanged(nameof(LanguageEnglishLabel));
                OnPropertyChanged(nameof(KeyStatusName));
                OnPropertyChanged(nameof(EngineStatusName));
                OnPropertyChanged(nameof(DatabaseStatusName));
                NotifyStatusPresentationChanged();
            }
        }
    }

    public bool IsChineseLanguage => string.Equals(UiLanguageCode, "zh-CN", StringComparison.OrdinalIgnoreCase);
    public bool IsEnglishLanguage => string.Equals(UiLanguageCode, "en-US", StringComparison.OrdinalIgnoreCase);
    public string ThemeSystemLabel => L("系统", "System");
    public string ThemeLightLabel => L("亮色", "Light");
    public string ThemeDarkLabel => L("暗色", "Dark");
    public string LanguageChineseLabel => "中";
    public string LanguageEnglishLabel => "EN";
    public string KeyStatusName => L("密钥状态", "Key status");
    public string EngineStatusName => L("音频引擎状态", "Audio engine status");
    public string DatabaseStatusName => L("数据库状态", "Database status");

    private int _activeKeySlot;
    public int ActiveKeySlot
    {
        get => _activeKeySlot;
        set
        {
            if (SetProperty(ref _activeKeySlot, value))
            {
                NotifyStatusPresentationChanged();
            }
        }
    }

    private readonly AppDatabase _database;
    private readonly TagMappingStore _tagStore;
    private readonly EvidenceStore _evidenceStore;
    private List<KeySlotSummary> _keySlotSummaries = [];

    private AppViewModel()
    {
        _database = new AppDatabase();
        _tagStore = new TagMappingStore(_database);
        _evidenceStore = new EvidenceStore(_database);
    }

    /// <summary>
    /// Gets the database instance.
    /// </summary>
    public AppDatabase Database => _database;

    /// <summary>
    /// Gets the tag mapping store.
    /// </summary>
    public TagMappingStore TagStore => _tagStore;

    /// <summary>
    /// Gets the evidence store.
    /// </summary>
    public EvidenceStore EvidenceStore => _evidenceStore;

    /// <summary>
    /// Initializes application state (call on startup).
    /// </summary>
    public async Task InitializeAsync()
    {
        await RefreshUiLanguageAsync();
        await RefreshRuntimeStatusAsync();
    }

    /// <summary>
    /// Sets current identity and refreshes global key status.
    /// NOTE: Key storage is global (not per-user in Rust FFI).
    /// </summary>
    public Task SetIdentityAsync(string identity)
    {
        CurrentIdentity = identity;
        return RefreshKeyStatusAsync();
    }

    /// <summary>
    /// Generates key in current active slot.
    /// </summary>
    [RelayCommand]
    private async Task GenerateKeyAsync()
    {
        var (key, error) = await Task.Run(AwmKeyBridge.GenerateAndSaveKey);
        if (error == AwmError.Ok && key is not null)
        {
            await RefreshKeyStatusAsync();
        }
    }

    /// <summary>
    /// Deletes key in current active slot.
    /// </summary>
    [RelayCommand]
    private async Task DeleteKeyAsync()
    {
        var (_, error) = await Task.Run(AwmKeyBridge.DeleteKey);
        if (error == AwmError.Ok)
        {
            await RefreshRuntimeStatusAsync();
        }
    }

    /// <summary>
    /// Refreshes runtime status shown in top navigation (key, engine, database).
    /// </summary>
    [RelayCommand]
    public async Task RefreshRuntimeStatusAsync()
    {
        if (!AwmNative.EnsureLoaded())
        {
            DatabaseAvailable = false;
            KeyAvailable = false;
            EngineAvailable = false;
            KeySourceLabel = L("本机引擎加载失败", "Native engine load failed");
            EnginePath = string.Empty;
            EngineBackend = "-";
            EngineEac3 = "unavailable";
            EngineContainers = "-";
            TotalTags = 0;
            TotalEvidence = 0;
            ActiveKeySlot = 0;
            NotifyStatusPresentationChanged();
            return;
        }

        await RefreshDatabaseStatusAsync();
        RefreshEngineStatus();
        await RefreshKeyStatusAsync();
        await RefreshActiveKeySlotAsync();
    }

    public Task RefreshUiLanguageAsync()
    {
        try
        {
            var (value, error) = AwmBridge.GetUiLanguage();
            if (error == AwmError.Ok && !string.IsNullOrWhiteSpace(value))
            {
                UiLanguageCode = NormalizeLanguageCode(value!);
                return Task.CompletedTask;
            }
        }
        catch
        {
            // Native layer unavailable: fall back to system UI culture.
        }

        var defaultCode = CultureInfo.CurrentUICulture.Name.StartsWith("zh", StringComparison.OrdinalIgnoreCase)
            ? "zh-CN"
            : "en-US";
        UiLanguageCode = defaultCode;
        return Task.CompletedTask;
    }

    public Task SetUiLanguageAsync(string code)
    {
        var normalized = NormalizeLanguageCode(code);
        try
        {
            var error = AwmBridge.SetUiLanguage(normalized);
            if (error == AwmError.Ok)
            {
                UiLanguageCode = normalized;
                return Task.CompletedTask;
            }
        }
        catch
        {
            return RefreshUiLanguageAsync();
        }

        return RefreshUiLanguageAsync();
    }

    /// <summary>
    /// Refreshes statistics (tags and evidence counts).
    /// </summary>
    [RelayCommand]
    public async Task RefreshStatsAsync()
    {
        await RefreshDatabaseStatusAsync();
        await RefreshKeyStatusAsync();
    }

    /// <summary>
    /// Refreshes global key availability and backend source label.
    /// </summary>
    public Task RefreshKeyStatusAsync()
    {
        try
        {
            var (summaries, summaryError) = AwmKeyBridge.GetSlotSummaries();
            _keySlotSummaries = summaryError == AwmError.Ok ? summaries : [];

            var exists = AwmKeyBridge.KeyExists();
            KeyAvailable = exists;
            if (!exists)
            {
                KeySourceLabel = L("未配置", "Not configured");
                return Task.CompletedTask;
            }

            var (backend, error) = AwmKeyBridge.GetBackendLabel();
            if (error == AwmError.Ok && !string.IsNullOrWhiteSpace(backend) && !string.Equals(backend, "none", StringComparison.OrdinalIgnoreCase))
            {
                KeySourceLabel = backend;
            }
            else
            {
                KeySourceLabel = L("已配置（来源未知）", "Configured (unknown backend)");
            }

            NotifyStatusPresentationChanged();
        }
        catch
        {
            _keySlotSummaries = [];
            KeyAvailable = false;
            KeySourceLabel = L("不可用", "Unavailable");
            NotifyStatusPresentationChanged();
        }
        return Task.CompletedTask;
    }

    private async Task RefreshDatabaseStatusAsync()
    {
        await Task.CompletedTask;
        try
        {
            var (tagCount, evidenceCount, error) = AwmDatabaseBridge.GetSummary();
            if (error != AwmError.Ok)
            {
                DatabaseAvailable = false;
                TotalTags = 0;
                TotalEvidence = 0;
                return;
            }

            DatabaseAvailable = true;
            TotalTags = (int)Math.Min(tagCount, int.MaxValue);
            TotalEvidence = (int)Math.Min(evidenceCount, int.MaxValue);
        }
        catch
        {
            DatabaseAvailable = false;
            TotalTags = 0;
            TotalEvidence = 0;
        }
    }

    public async Task RefreshActiveKeySlotAsync()
    {
        await Task.CompletedTask;
        var (slot, error) = AwmKeyBridge.GetActiveSlot();
        ActiveKeySlot = error == AwmError.Ok ? Math.Clamp(slot, 0, 31) : 0;
    }

    public async Task SetActiveKeySlotAsync(int slot)
    {
        await Task.CompletedTask;
        var error = AwmKeyBridge.SetActiveSlot(slot);
        if (error == AwmError.Ok)
        {
            ActiveKeySlot = Math.Clamp(slot, 0, 31);
            await RefreshKeyStatusAsync();
            return;
        }

        var (currentSlot, _) = AwmKeyBridge.GetActiveSlot();
        ActiveKeySlot = Math.Clamp(currentSlot, 0, 31);
        await RefreshKeyStatusAsync();
    }

    private void RefreshEngineStatus()
    {
        try
        {
            var (path, error) = AwmBridge.GetAudioBinaryPath();
            EnginePath = path ?? string.Empty;
            EngineAvailable = error == AwmError.Ok && !string.IsNullOrEmpty(path);
            var (caps, capsError) = AwmBridge.GetAudioMediaCapabilities();
            if (capsError == AwmError.Ok && caps is not null)
            {
                EngineBackend = caps.Value.Backend;
                EngineEac3 = caps.Value.Eac3Decode ? "available" : "unavailable";
                var containers = new List<string>();
                if (caps.Value.ContainerMp4) containers.Add("mp4");
                if (caps.Value.ContainerMkv) containers.Add("mkv");
                if (caps.Value.ContainerTs) containers.Add("ts");
                EngineContainers = containers.Count == 0 ? "-" : string.Join(",", containers);
            }
            else
            {
                EngineBackend = "-";
                EngineEac3 = "unavailable";
                EngineContainers = "-";
            }
        }
        catch
        {
            EngineAvailable = false;
            EnginePath = string.Empty;
            EngineBackend = "-";
            EngineEac3 = "unavailable";
            EngineContainers = "-";
        }
    }

    private static Brush GetStatusBrush(string resourceKey)
    {
        var resources = Application.Current.Resources;
        if (resources.TryGetValue(resourceKey, out var value) && value is Brush brush)
        {
            return brush;
        }

        if (resources.TryGetValue("TextFillColorSecondaryBrush", out var fallback) && fallback is Brush fallbackBrush)
        {
            return fallbackBrush;
        }

        if (resources.TryGetValue("NeutralBrush", out var neutralFallback) && neutralFallback is Brush neutralBrush)
        {
            return neutralBrush;
        }

        return new SolidColorBrush(Microsoft.UI.Colors.Transparent);
    }

    private static Brush GetAvailabilityBrush(bool available)
    {
        return GetStatusBrush(available ? "SuccessBrush" : "ErrorBrush");
    }

    private void NotifyStatusPresentationChanged()
    {
        OnPropertyChanged(nameof(KeyStatusBrush));
        OnPropertyChanged(nameof(EngineStatusBrush));
        OnPropertyChanged(nameof(DatabaseStatusBrush));
        OnPropertyChanged(nameof(KeyStatusTooltip));
        OnPropertyChanged(nameof(EngineStatusTooltip));
        OnPropertyChanged(nameof(DatabaseStatusTooltip));
    }

    private string BuildKeyStatusTooltip()
    {
        var configured = _keySlotSummaries
            .Where(item => item.HasKey)
            .OrderBy(item => item.Slot)
            .ToList();
        var active = _keySlotSummaries.FirstOrDefault(item => item.Slot == ActiveKeySlot);
        var activeKeyId = active?.KeyId ?? L("未配置", "Not configured");

        var digest = configured.Count == 0
            ? "-"
            : string.Join(", ", configured.Take(6).Select(item => $"{item.Slot}:{item.KeyId ?? "-"}"));
        if (configured.Count > 6)
        {
            digest += ", ...";
        }

        var duplicateSlots = configured
            .Where(item => string.Equals(item.StatusText, "duplicate", StringComparison.OrdinalIgnoreCase))
            .Select(item => item.Slot)
            .Distinct()
            .OrderBy(slot => slot)
            .ToArray();

        var lines = new List<string>
        {
            $"{L("激活槽位", "Active slot")}：{ActiveKeySlot}",
            $"{L("激活 Key ID", "Active Key ID")}：{activeKeyId}",
            $"{L("已配置槽位", "Configured slots")}：{configured.Count}/32",
            $"{L("槽位摘要", "Slot summary")}：{digest}"
        };
        if (duplicateSlots.Length > 0)
        {
            lines.Add($"{L("重复密钥槽位", "Duplicate key slots")}：{string.Join(",", duplicateSlots)}");
        }
        lines.Add(KeyAvailable
            ? L("点击刷新状态", "Click to refresh status")
            : L("未配置密钥，请前往“密钥”页面生成", "No key configured. Open Key page to create one."));
        return string.Join('\n', lines);
    }

    private static string NormalizeLanguageCode(string? value)
    {
        return string.Equals(value?.Trim(), "en-US", StringComparison.OrdinalIgnoreCase)
            ? "en-US"
            : "zh-CN";
    }

    private string L(string zh, string en) => IsEnglishLanguage ? en : zh;
}
