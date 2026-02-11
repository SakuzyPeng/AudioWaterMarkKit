using AWMKit.Data;
using AWMKit.Models;
using AWMKit.Native;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Media;
using System;
using System.Collections.Generic;
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
        ? $"音频引擎：可用\n路径：{EnginePath}\n点击刷新状态"
        : "音频引擎：不可用\n请检查 bundled 或 PATH\n点击刷新状态";

    public string DatabaseStatusTooltip => DatabaseAvailable
        ? $"数据库：可用\n映射：{TotalTags}\n证据：{TotalEvidence}\n点击刷新状态"
        : "数据库：不可用\n点击刷新状态";

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
        await RefreshDatabaseStatusAsync();
        RefreshEngineStatus();
        await RefreshKeyStatusAsync();
        await RefreshActiveKeySlotAsync();
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
        var (summaries, summaryError) = AwmKeyBridge.GetSlotSummaries();
        _keySlotSummaries = summaryError == AwmError.Ok ? summaries : [];

        var exists = AwmKeyBridge.KeyExists();
        KeyAvailable = exists;
        if (!exists)
        {
            KeySourceLabel = "未配置";
            return Task.CompletedTask;
        }

        var (backend, error) = AwmKeyBridge.GetBackendLabel();
        if (error == AwmError.Ok && !string.IsNullOrWhiteSpace(backend) && !string.Equals(backend, "none", StringComparison.OrdinalIgnoreCase))
        {
            KeySourceLabel = backend;
        }
        else
        {
            KeySourceLabel = "已配置（来源未知）";
        }

        NotifyStatusPresentationChanged();
        return Task.CompletedTask;
    }

    private async Task RefreshDatabaseStatusAsync()
    {
        await Task.CompletedTask;
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
        }
        catch
        {
            EngineAvailable = false;
            EnginePath = string.Empty;
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
        var activeKeyId = active?.KeyId ?? "未配置";

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
            $"激活槽位：{ActiveKeySlot}",
            $"激活 Key ID：{activeKeyId}",
            $"已配置槽位：{configured.Count}/32",
            $"槽位摘要：{digest}"
        };
        if (duplicateSlots.Length > 0)
        {
            lines.Add($"重复密钥槽位：{string.Join(",", duplicateSlots)}");
        }
        lines.Add(KeyAvailable ? "点击刷新状态" : "未配置密钥，请前往“密钥”页面生成");
        return string.Join('\n', lines);
    }
}
