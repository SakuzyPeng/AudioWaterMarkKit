using AWMKit.Data;
using AWMKit.Native;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using System;
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

    [ObservableProperty]
    private bool _keyAvailable;

    [ObservableProperty]
    private bool _engineAvailable;

    [ObservableProperty]
    private bool _databaseAvailable;

    [ObservableProperty]
    private string _currentIdentity = string.Empty;

    [ObservableProperty]
    private string _enginePath = string.Empty;

    [ObservableProperty]
    private int _totalTags;

    [ObservableProperty]
    private int _totalEvidence;

    private readonly AppDatabase _database;
    private readonly TagMappingStore _tagStore;
    private readonly EvidenceStore _evidenceStore;

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
        await Task.Run(async () =>
        {
            // Check database
            DatabaseAvailable = await _database.OpenAsync();

            // Check engine
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

            // Load statistics
            if (DatabaseAvailable)
            {
                var tags = await _tagStore.ListAllAsync();
                TotalTags = tags.Count;
                TotalEvidence = await _evidenceStore.CountAsync();
            }
        });
    }

    /// <summary>
    /// Sets the current user identity and checks key availability.
    /// NOTE: Key storage is global (not per-user in Rust FFI).
    /// </summary>
    public async Task SetIdentityAsync(string identity)
    {
        await Task.Run(() =>
        {
            CurrentIdentity = identity;
            // Check global key availability (Rust KeyStore is global)
            KeyAvailable = !string.IsNullOrEmpty(identity) && AwmKeyBridge.KeyExists();
        });
    }

    /// <summary>
    /// Generates a new global key.
    /// NOTE: This is a GLOBAL key, not per-user.
    /// </summary>
    [RelayCommand]
    private async Task GenerateKeyAsync()
    {
        if (string.IsNullOrEmpty(CurrentIdentity))
        {
            return;
        }

        await Task.Run(() =>
        {
            var (key, error) = AwmKeyBridge.GenerateAndSaveKey();
            KeyAvailable = error == AwmError.Ok && key is not null;
        });
    }

    /// <summary>
    /// Deletes the global key.
    /// NOTE: This deletes the GLOBAL key, affecting all users.
    /// </summary>
    [RelayCommand]
    private async Task DeleteKeyAsync()
    {
        if (string.IsNullOrEmpty(CurrentIdentity))
        {
            return;
        }

        await Task.Run(() =>
        {
            var error = AwmKeyBridge.DeleteKey();
            KeyAvailable = error != AwmError.Ok;
        });
    }

    /// <summary>
    /// Refreshes statistics (tags and evidence counts).
    /// </summary>
    [RelayCommand]
    public async Task RefreshStatsAsync()
    {
        if (!DatabaseAvailable)
        {
            return;
        }

        await Task.Run(async () =>
        {
            var tags = await _tagStore.ListAllAsync();
            TotalTags = tags.Count;
            TotalEvidence = await _evidenceStore.CountAsync();
        });
    }
}
