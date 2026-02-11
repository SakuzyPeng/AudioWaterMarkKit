using AWMKit.Models;
using AWMKit.Native;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Media;
using System;
using System.Collections.Generic;
using System.Collections.ObjectModel;
using System.Collections.Specialized;
using System.ComponentModel;
using System.IO;
using System.Linq;
using System.Security.Cryptography;
using System.Text;
using System.Threading;
using System.Threading.Tasks;

namespace AWMKit.ViewModels;

/// <summary>
/// Embed page state model aligned with macOS behavior.
/// </summary>
public sealed partial class EmbedViewModel : ObservableObject
{
    private const int MaxLogCount = 200;
    private static readonly char[] SuggestedIdentityCharset = "ABCDEFGHJKMNPQRSTUVWXYZ23456789_".ToCharArray();

    private readonly HashSet<string> _supportedAudioExtensions = new(StringComparer.OrdinalIgnoreCase)
    {
        ".wav",
        ".flac",
        ".m4a",
        ".alac",
    };

    private CancellationTokenSource? _embedCts;
    private CancellationTokenSource? _progressResetCts;
    private bool _isUpdatingFromSelection;
    private readonly AppViewModel _appState = AppViewModel.Instance;

    private bool _isInputSelectSuccess;
    public bool IsInputSelectSuccess
    {
        get => _isInputSelectSuccess;
        private set => SetProperty(ref _isInputSelectSuccess, value);
    }

    private bool _isOutputSelectSuccess;
    public bool IsOutputSelectSuccess
    {
        get => _isOutputSelectSuccess;
        private set => SetProperty(ref _isOutputSelectSuccess, value);
    }

    private bool _isEmbedSuccess;
    public bool IsEmbedSuccess
    {
        get => _isEmbedSuccess;
        private set
        {
            if (SetProperty(ref _isEmbedSuccess, value))
            {
                OnPropertyChanged(nameof(ShowEmbedDefaultPlayIcon));
                OnPropertyChanged(nameof(ShowEmbedSuccessPlayIcon));
            }
        }
    }

    private string? _inputSource;
    public string? InputSource
    {
        get => _inputSource;
        private set
        {
            if (SetProperty(ref _inputSource, value))
            {
                OnPropertyChanged(nameof(InputSourceText));
            }
        }
    }

    public string InputSourceText => string.IsNullOrWhiteSpace(InputSource) ? L("尚未选择输入源", "No input source selected") : InputSource;

    private string? _outputDirectory;
    public string? OutputDirectory
    {
        get => _outputDirectory;
        set
        {
            if (SetProperty(ref _outputDirectory, NormalizePathOrNull(value)))
            {
                OnPropertyChanged(nameof(OutputDirectoryText));
            }
        }
    }

    public string OutputDirectoryText => string.IsNullOrWhiteSpace(OutputDirectory) ? L("默认写回各文件所在目录", "Default: write back to source directory") : OutputDirectory;

    private string _usernameInput = string.Empty;
    public string UsernameInput
    {
        get => _usernameInput;
        set
        {
            if (SetProperty(ref _usernameInput, value))
            {
                UpdateMappingSuggestions();
                OnPropertyChanged(nameof(PreviewTagText));
                OnPropertyChanged(nameof(PreviewTagDisplay));
                OnPropertyChanged(nameof(MatchedMappingHintText));
                OnPropertyChanged(nameof(ReuseHintText));
                OnPropertyChanged(nameof(PreviewTagBrush));
            }
        }
    }

    private EmbedMappingOption? _selectedMapping;
    public EmbedMappingOption? SelectedMapping
    {
        get => _selectedMapping;
        set
        {
            if (!SetProperty(ref _selectedMapping, value))
            {
                return;
            }

            if (_isUpdatingFromSelection || value is null)
            {
                return;
            }

            _isUpdatingFromSelection = true;
            try
            {
                UsernameInput = value.Username;
            }
            finally
            {
                _isUpdatingFromSelection = false;
            }
        }
    }

    private int _strength = 10;
    public int Strength
    {
        get => _strength;
        set => SetProperty(ref _strength, Math.Clamp(value, 1, 30));
    }

    private string _customSuffix = "_wm";
    public string CustomSuffix
    {
        get => _customSuffix;
        set => SetProperty(ref _customSuffix, value);
    }

    public ObservableCollection<ChannelLayoutOption> ChannelLayoutOptions { get; } = BuildChannelLayoutOptions();

    private ChannelLayoutOption? _selectedChannelLayout;
    public ChannelLayoutOption? SelectedChannelLayout
    {
        get => _selectedChannelLayout;
        set => SetProperty(ref _selectedChannelLayout, value);
    }

    private bool _isProcessing;
    public bool IsProcessing
    {
        get => _isProcessing;
        private set
        {
            if (SetProperty(ref _isProcessing, value))
            {
                OnPropertyChanged(nameof(CanEmbedOrStop));
                OnPropertyChanged(nameof(EmbedButtonText));
                OnPropertyChanged(nameof(ShowEmbedStopIcon));
                OnPropertyChanged(nameof(ShowEmbedDefaultPlayIcon));
                OnPropertyChanged(nameof(ShowEmbedSuccessPlayIcon));
            }
        }
    }

    private bool _isCancelling;
    public bool IsCancelling
    {
        get => _isCancelling;
        private set => SetProperty(ref _isCancelling, value);
    }

    private double _progress;
    public double Progress
    {
        get => _progress;
        private set
        {
            if (SetProperty(ref _progress, value))
            {
                OnPropertyChanged(nameof(ProgressPercent));
                OnPropertyChanged(nameof(ProgressPercentText));
            }
        }
    }

    public double ProgressPercent => Math.Clamp(Progress * 100.0, 0, 100);
    public string ProgressPercentText => $"{(int)ProgressPercent}%";

    private int _currentProcessingIndex = -1;
    public int CurrentProcessingIndex
    {
        get => _currentProcessingIndex;
        private set => SetProperty(ref _currentProcessingIndex, value);
    }

    private string? _currentProcessingFile;
    public string? CurrentProcessingFile
    {
        get => _currentProcessingFile;
        private set => SetProperty(ref _currentProcessingFile, value);
    }

    private bool _isClearQueueSuccess;
    public bool IsClearQueueSuccess
    {
        get => _isClearQueueSuccess;
        private set => SetProperty(ref _isClearQueueSuccess, value);
    }

    private bool _isClearLogsSuccess;
    public bool IsClearLogsSuccess
    {
        get => _isClearLogsSuccess;
        private set => SetProperty(ref _isClearLogsSuccess, value);
    }

    public Brush PreviewTagBrush =>
        MatchedMappingForInput is not null
            ? ThemeSuccessBrush()
            : (PreviewTagText == "-" ? ThemeSecondaryBrush() : ThemePrimaryBrush());

    public ObservableCollection<string> SelectedFiles { get; } = new();
    public ObservableCollection<LogEntry> Logs { get; } = new();
    public ObservableCollection<EmbedMappingOption> AllMappings { get; } = new();
    public ObservableCollection<EmbedMappingOption> MappingSuggestions { get; } = new();
    public ObservableCollection<string> PendingForceReviewFiles { get; } = new();

    private int _forceReviewPromptVersion;
    public int ForceReviewPromptVersion
    {
        get => _forceReviewPromptVersion;
        private set => SetProperty(ref _forceReviewPromptVersion, value);
    }

    public int PendingForceReviewCount => PendingForceReviewFiles.Count;
    public bool HasPendingForceReview => PendingForceReviewCount > 0;

    public int QueueCount => SelectedFiles.Count;
    public bool HasQueueCount => QueueCount > 0;
    public string QueueCountText => L($"共 {QueueCount} 个", $"Total {QueueCount}");
    public bool HasQueueFiles => QueueCount > 0;
    public bool ShowQueueEmptyHint => !HasQueueFiles;

    private bool _isKeyAvailable;
    public bool IsKeyAvailable
    {
        get => _isKeyAvailable;
        set
        {
            if (SetProperty(ref _isKeyAvailable, value))
            {
                OnPropertyChanged(nameof(CanEmbedOrStop));
            }
        }
    }

    public bool HasLogs => Logs.Count > 0;
    public string LogCountText => L($"共 {Logs.Count} 条", $"Total {Logs.Count}");
    public bool ShowNoLogsHint => !HasLogs;

    public bool HasMappings => AllMappings.Count > 0;
    public bool CanEmbedOrStop => IsKeyAvailable && (IsProcessing || SelectedFiles.Count > 0);
    public string EmbedButtonText => IsProcessing ? L("停止", "Stop") : L("嵌入", "Embed");
    public bool ShowEmbedStopIcon => IsProcessing;
    public bool ShowEmbedDefaultPlayIcon => !IsProcessing && !IsEmbedSuccess;
    public bool ShowEmbedSuccessPlayIcon => !IsProcessing && IsEmbedSuccess;
    public string InputSourceLabel => L("输入源", "Input source");
    public string OutputDirectoryLabel => L("输出目录", "Output directory");
    public string MissingKeyMessage => L("未配置密钥，请前往密钥页完成生成。", "No key configured. Please go to Key page to create one.");
    public string GoToKeyPageText => L("前往密钥页", "Go to Key page");
    public string SelectActionText => L("选择", "Select");
    public string ClearActionText => L("清空", "Clear");
    public string SettingsTitle => L("嵌入设置", "Embed settings");
    public string UsernameLabel => L("用户名", "Username");
    public string UsernamePlaceholder => L("例如: user_001", "e.g. user_001");
    public string StoredMappingsLabel => L("已存储的映射", "Stored mappings");
    public string StrengthLabel => L("水印强度", "Watermark strength");
    public string SuffixLabel => L("输出后缀", "Output suffix");
    public string LayoutLabel => L("声道布局", "Channel layout");
    public string DropZoneTitle => L("拖拽音频文件到此处", "Drag audio files here");
    public string DropZoneSubtitle => L("支持 WAV / FLAC / M4A / ALAC 格式，可批量拖入", "Supports WAV / FLAC / M4A / ALAC, batch drop enabled");
    public string QueueTitle => L("待处理文件", "Pending files");
    public string QueueEmptyText => L("暂无文件", "No files");
    public string LogsTitle => L("事件日志", "Event logs");
    public string LogsEmptyText => L("暂无日志", "No logs");
    public string SelectInputSourceAccessibility => L("选择输入源", "Select input source");
    public string SelectOutputDirectoryAccessibility => L("选择输出目录", "Select output directory");
    public string EmbedActionAccessibility => L("开始或停止嵌入", "Start or stop embedding");
    public string ClearQueueAccessibility => L("清空队列", "Clear queue");
    public string ClearLogsAccessibility => L("清空日志", "Clear logs");

    public string PreviewTagText => ResolveTagValue() ?? "-";
    public string PreviewTagDisplay => $"Tag: {PreviewTagText}";
    public string? MatchedMappingHintText => MatchedMappingForInput is null ? null : L("已存在映射，自动复用", "Existing mapping found, auto reused");
    public string ReuseHintText => MatchedMappingForInput is null ? string.Empty : L("复用 ", "Reusing ");
    public string MappingPlaceholderText => HasMappings ? L("选择已存储映射", "Select stored mapping") : L("暂无已存储映射", "No stored mappings");
    public string ForceReviewDialogTitle => L("检测到已有水印", "Existing watermark detected");
    public string ForceReviewDialogPrimaryText => L("强行嵌入", "Force embed");
    public string ForceReviewDialogSecondaryText => L("移出队列", "Remove from queue");
    public string ForceReviewDialogCloseText => L("稍后处理", "Later");
    public string ForceReviewDialogMessage
    {
        get
        {
            if (PendingForceReviewFiles.Count == 0)
            {
                return L("当前没有待确认文件。", "No pending files.");
            }

            var preview = string.Join("、", PendingForceReviewFiles.Take(3).Select(Path.GetFileName));
            if (PendingForceReviewFiles.Count <= 3)
            {
                return L(
                    $"检测到 {PendingForceReviewFiles.Count} 个已含水印文件：{preview}",
                    $"Detected {PendingForceReviewFiles.Count} already-watermarked files: {preview}");
            }

            var remain = PendingForceReviewFiles.Count - 3;
            return L(
                $"检测到 {PendingForceReviewFiles.Count} 个已含水印文件：{preview} 等 {remain} 个",
                $"Detected {PendingForceReviewFiles.Count} already-watermarked files: {preview} and {remain} more");
        }
    }

    private EmbedMappingOption? MatchedMappingForInput
    {
        get
        {
            var username = NormalizedUsernameInput();
            if (string.IsNullOrEmpty(username))
            {
                return null;
            }

            return AllMappings.FirstOrDefault(option =>
                string.Equals(option.Username, username, StringComparison.OrdinalIgnoreCase));
        }
    }

    public EmbedViewModel()
    {
        SelectedChannelLayout = ChannelLayoutOptions.FirstOrDefault();
        SelectedFiles.CollectionChanged += OnSelectedFilesChanged;
        Logs.CollectionChanged += OnLogsChanged;
        AllMappings.CollectionChanged += OnMappingsChanged;
        PendingForceReviewFiles.CollectionChanged += OnPendingForceReviewFilesChanged;
        _appState.PropertyChanged += OnAppStatePropertyChanged;
        _ = RefreshTagMappingsAsync();
    }

    private void OnAppStatePropertyChanged(object? sender, PropertyChangedEventArgs e)
    {
        if (e.PropertyName != nameof(AppViewModel.UiLanguageCode))
        {
            return;
        }

        OnPropertyChanged(nameof(InputSourceText));
        OnPropertyChanged(nameof(OutputDirectoryText));
        OnPropertyChanged(nameof(QueueCountText));
        OnPropertyChanged(nameof(LogCountText));
        OnPropertyChanged(nameof(EmbedButtonText));
        OnPropertyChanged(nameof(MatchedMappingHintText));
        OnPropertyChanged(nameof(ReuseHintText));
        OnPropertyChanged(nameof(MappingPlaceholderText));
        OnPropertyChanged(nameof(ForceReviewDialogTitle));
        OnPropertyChanged(nameof(ForceReviewDialogPrimaryText));
        OnPropertyChanged(nameof(ForceReviewDialogSecondaryText));
        OnPropertyChanged(nameof(ForceReviewDialogCloseText));
        OnPropertyChanged(nameof(ForceReviewDialogMessage));
        NotifyLocalizedTextChanged();
        RebuildLayoutOptions();
    }

    public async Task RefreshTagMappingsAsync()
    {
        var mappings = await AppViewModel.Instance.TagStore.ListAllAsync();
        var options = mappings
            .Select(x => new EmbedMappingOption
            {
                Username = x.Username,
                Tag = x.Tag
            })
            .OrderBy(x => x.Username, StringComparer.OrdinalIgnoreCase)
            .ToList();

        AllMappings.Clear();
        foreach (var option in options)
        {
            AllMappings.Add(option);
        }

        UpdateMappingSuggestions();
        OnPropertyChanged(nameof(PreviewTagText));
        OnPropertyChanged(nameof(PreviewTagDisplay));
        OnPropertyChanged(nameof(MatchedMappingHintText));
        OnPropertyChanged(nameof(ReuseHintText));
        OnPropertyChanged(nameof(PreviewTagBrush));
    }

    public void SetInputSource(string sourcePath)
    {
        if (string.IsNullOrWhiteSpace(sourcePath))
        {
            return;
        }

        InputSource = sourcePath;
        var files = ResolveAudioFiles(sourcePath);
        AppendFilesWithDedup(files);
        _ = FlashInputSelectAsync();
    }

    public void AddDroppedFiles(IEnumerable<string> filePaths)
    {
        var resolved = new List<string>();
        foreach (var path in filePaths)
        {
            if (Directory.Exists(path))
            {
                resolved.AddRange(ResolveAudioFiles(path));
            }
            else if (IsSupportedAudioFile(path))
            {
                resolved.Add(path);
            }
        }

        AppendFilesWithDedup(resolved);
    }

    [RelayCommand]
    private void SelectMapping(EmbedMappingOption? option)
    {
        if (option is null)
        {
            return;
        }

        SelectedMapping = option;
        UsernameInput = option.Username;
    }

    [RelayCommand]
    private void RemoveQueueFile(string? filePath)
    {
        if (string.IsNullOrWhiteSpace(filePath))
        {
            return;
        }

        RemovePendingFileByKey(NormalizedPathKey(filePath));
        SelectedFiles.Remove(filePath);
    }

    [RelayCommand]
    private async Task ClearQueueAsync()
    {
        if (!SelectedFiles.Any())
        {
            AddLog(L("队列为空", "Queue is empty"), L("没有可移除的文件", "No files to remove"), true, true, LogIconTone.Info);
            return;
        }

        var count = SelectedFiles.Count;
        SelectedFiles.Clear();
        PendingForceReviewFiles.Clear();
        AddLog(L("已清空队列", "Queue cleared"), L($"移除了 {count} 个文件", $"Removed {count} files"), true, false, LogIconTone.Success, LogKind.QueueCleared);
        await FlashClearQueueAsync();
    }

    [RelayCommand]
    private async Task ClearLogsAsync()
    {
        if (!Logs.Any())
        {
            AddLog(L("日志为空", "Logs are empty"), L("没有可清空的日志", "No logs to clear"), true, true, LogIconTone.Info);
            return;
        }

        var count = Logs.Count;
        Logs.Clear();
        AddLog(L("已清空日志", "Logs cleared"), L($"移除了 {count} 条日志记录", $"Removed {count} log entries"), true, true, LogIconTone.Success, LogKind.LogsCleared);
        await FlashClearLogsAsync();
    }

    public void KeepPendingForceInQueue()
    {
        if (!PendingForceReviewFiles.Any())
        {
            return;
        }

        AddLog(
            L("已保留待确认文件", "Pending files kept"),
            L("待确认文件仍保留在队列中，可稍后决定是否强行嵌入", "Pending files remain in queue; you can decide force embed later"),
            false,
            true,
            LogIconTone.Warning);
    }

    public void RemovePendingForceFromQueue()
    {
        if (!PendingForceReviewFiles.Any())
        {
            return;
        }

        var pendingKeys = new HashSet<string>(PendingForceReviewFiles.Select(NormalizedPathKey), StringComparer.OrdinalIgnoreCase);
        var before = SelectedFiles.Count;
        for (var i = SelectedFiles.Count - 1; i >= 0; i--)
        {
            if (pendingKeys.Contains(NormalizedPathKey(SelectedFiles[i])))
            {
                SelectedFiles.RemoveAt(i);
            }
        }

        var removed = before - SelectedFiles.Count;
        var reviewed = PendingForceReviewFiles.Count;
        PendingForceReviewFiles.Clear();
        AddLog(
            L("已移出待确认文件", "Pending files removed"),
            L($"已移出 {removed} 个文件（待确认 {reviewed} 个）", $"Removed {removed} files ({reviewed} pending reviewed)"),
            true,
            false,
            LogIconTone.Success,
            LogKind.QueueCleared);
    }

    public async Task ForceEmbedPendingAsync()
    {
        if (!PendingForceReviewFiles.Any())
        {
            return;
        }

        await EmbedAsync(forceTargets: PendingForceReviewFiles.ToList());
    }

    private void RequestForceReviewPrompt()
    {
        ForceReviewPromptVersion += 1;
    }

    [RelayCommand]
    private async Task EmbedOrStopAsync()
    {
        if (IsProcessing)
        {
            _embedCts?.Cancel();
            IsCancelling = true;
            AddLog(L("嵌入已停止", "Embedding stopped"), L("用户手动停止", "Stopped by user"), false, true, LogIconTone.Warning);
            return;
        }

        await EmbedAsync();
    }

    private async Task EmbedAsync(IReadOnlyList<string>? forceTargets = null)
    {
        if (IsProcessing)
        {
            return;
        }

        var isForcedPass = forceTargets is not null;
        if (!isForcedPass && PendingForceReviewFiles.Any())
        {
            RequestForceReviewPrompt();
            return;
        }

        if (!SelectedFiles.Any())
        {
            AddLog(L("队列为空", "Queue is empty"), L("请先添加音频文件", "Add audio files first"), false, true, LogIconTone.Warning);
            return;
        }

        var username = NormalizedUsernameInput();
        if (string.IsNullOrEmpty(username))
        {
            AddLog(L("用户名未填写", "Username is missing"), L("请输入用户名以自动生成 Tag", "Enter username to generate tag automatically"), false, true, LogIconTone.Warning);
            return;
        }

        var resolvedTag = ResolveTagValue();
        if (string.IsNullOrWhiteSpace(resolvedTag))
        {
            AddLog(L("Tag 生成失败", "Tag generation failed"), L("请检查用户名输入", "Please check username input"), false, false, LogIconTone.Error);
            return;
        }

        var (key, keyError) = AwmKeyBridge.LoadKey();
        if (key is null || keyError != AwmError.Ok)
        {
            AddLog(L("嵌入失败", "Embed failed"), $"{L("密钥不可用", "Key unavailable")}: {keyError}", false, false, LogIconTone.Error);
            return;
        }

        var (message, encodeError) = AwmBridge.EncodeMessage(resolvedTag, key);
        if (message is null || encodeError != AwmError.Ok)
        {
            AddLog(L("嵌入失败", "Embed failed"), $"{L("消息编码失败", "Message encode failed")}: {encodeError}", false, false, LogIconTone.Error);
            return;
        }

        _progressResetCts?.Cancel();
        _embedCts = new CancellationTokenSource();
        var token = _embedCts.Token;

        IsProcessing = true;
        IsCancelling = false;
        Progress = 0;
        CurrentProcessingIndex = 0;

        var layout = SelectedChannelLayout?.Layout ?? AwmChannelLayout.Auto;
        var layoutText = SelectedChannelLayout?.DisplayText ?? L("自动", "Auto");
        if (isForcedPass)
        {
            AddLog(
                L("开始强行嵌入", "Force embed started"),
                L($"准备处理 {forceTargets!.Count} 个待确认文件", $"Preparing {forceTargets.Count} reviewed files for force embed"),
                false,
                false,
                LogIconTone.Warning);
        }
        else
        {
            AddLog(
                L("开始处理", "Processing started"),
                L($"准备处理 {SelectedFiles.Count} 个文件（{layoutText}）", $"Preparing to process {SelectedFiles.Count} files ({layoutText})"),
                true,
                false,
                LogIconTone.Info);
        }

        var forceTargetKeys = forceTargets is null
            ? null
            : new HashSet<string>(forceTargets.Select(NormalizedPathKey), StringComparer.OrdinalIgnoreCase);
        var initialTotal = Math.Max(forceTargetKeys?.Count ?? SelectedFiles.Count, 1);
        var successCount = 0;
        var failureCount = 0;
        var deferredFiles = new List<string>();
        var deferredKeys = new HashSet<string>(StringComparer.OrdinalIgnoreCase);
        var legacyFfiWarned = false;

        for (var processed = 0; processed < initialTotal; processed++)
        {
            if (token.IsCancellationRequested)
            {
                break;
            }

            var queueIndex = FindNextQueueIndex(forceTargetKeys);
            if (queueIndex < 0 || queueIndex >= SelectedFiles.Count)
            {
                break;
            }

            var inputPath = SelectedFiles[queueIndex];
            var inputKey = NormalizedPathKey(inputPath);
            CurrentProcessingFile = Path.GetFileName(inputPath);
            CurrentProcessingIndex = queueIndex;

            if (!isForcedPass)
            {
                (bool detected, AwmError error) precheckResult;
                try
                {
                    precheckResult = await RunCancelableNativeCallAsync(() =>
                    {
                        var detect = AwmBridge.DetectAudioMultichannelDetailed(inputPath, layout);
                        if (detect.error == AwmError.Ok && detect.result is not null)
                        {
                            return (true, AwmError.Ok);
                        }

                        if (detect.error == AwmError.NoWatermarkFound)
                        {
                            return (false, AwmError.Ok);
                        }

                        return (false, detect.error);
                    }, token);
                }
                catch (OperationCanceledException)
                {
                    break;
                }
                catch (Exception ex)
                {
                    AddLog(
                        $"{L("失败", "Failed")}: {Path.GetFileName(inputPath)}",
                        $"{L("预检异常", "Precheck exception")}: {ex.Message}",
                        false,
                        false,
                        LogIconTone.Error,
                        LogKind.ResultError);
                    failureCount += 1;
                    if (queueIndex < SelectedFiles.Count)
                    {
                        SelectedFiles.RemoveAt(queueIndex);
                    }
                    Progress = (processed + 1) / (double)initialTotal;
                    await Task.Yield();
                    continue;
                }

                if (precheckResult.error != AwmError.Ok)
                {
                    AddLog(
                        $"{L("失败", "Failed")}: {Path.GetFileName(inputPath)}",
                        $"{L("预检失败", "Precheck failed")}: {precheckResult.error}",
                        false,
                        false,
                        LogIconTone.Error,
                        LogKind.ResultError);
                    failureCount += 1;
                    if (queueIndex < SelectedFiles.Count)
                    {
                        SelectedFiles.RemoveAt(queueIndex);
                    }
                    Progress = (processed + 1) / (double)initialTotal;
                    await Task.Yield();
                    continue;
                }

                if (precheckResult.detected)
                {
                    if (queueIndex < SelectedFiles.Count)
                    {
                        var deferred = SelectedFiles[queueIndex];
                        SelectedFiles.RemoveAt(queueIndex);
                        SelectedFiles.Add(deferred);
                        if (deferredKeys.Add(inputKey))
                        {
                            deferredFiles.Add(deferred);
                        }
                    }

                    AddLog(
                        L("检测到已有水印", "Existing watermark detected"),
                        L($"{Path.GetFileName(inputPath)} 已移至队尾，等待汇总确认", $"{Path.GetFileName(inputPath)} moved to queue tail, awaiting review"),
                        false,
                        false,
                        LogIconTone.Warning,
                        LogKind.ResultNotFound);
                    Progress = (processed + 1) / (double)initialTotal;
                    await Task.Yield();
                    continue;
                }
            }

            string outputPath;
            try
            {
                outputPath = BuildOutputPath(inputPath);
            }
            catch (Exception ex)
            {
                AddLog($"{L("失败", "Failed")}: {Path.GetFileName(inputPath)}", ex.Message, false, false, LogIconTone.Error, LogKind.ResultError);
                failureCount += 1;
                if (queueIndex < SelectedFiles.Count)
                {
                    SelectedFiles.RemoveAt(queueIndex);
                }
                Progress = (processed + 1) / (double)initialTotal;
                await Task.Yield();
                continue;
            }

            (AwmError embedError, AwmError evidenceError, AwmBridge.EmbedEvidenceResult? snrResult) stepResult;
            try
            {
                stepResult = await RunCancelableNativeCallAsync(() =>
                {
                    var embed = AwmBridge.EmbedAudioMultichannel(
                        inputPath,
                        outputPath,
                        message,
                        layout,
                        Strength);

                    var evidence = AwmError.Ok;
                    AwmBridge.EmbedEvidenceResult? snrResult = null;
                    if (embed == AwmError.Ok)
                    {
                        var record = AwmBridge.RecordEmbedEvidence(
                            inputPath,
                            outputPath,
                            message,
                            key,
                            isForcedPass);
                        evidence = record.error;
                        snrResult = record.result;
                    }

                    return (embed, evidence, snrResult);
                }, token);
            }
            catch (OperationCanceledException)
            {
                break;
            }
            catch (Exception ex)
            {
                failureCount += 1;
                AddLog(
                    $"{L("失败", "Failed")}: {Path.GetFileName(inputPath)}",
                    $"{L("嵌入异常", "Embed exception")}: {ex.Message}",
                    false,
                    false,
                    LogIconTone.Error,
                    LogKind.ResultError);

                var removeOnException = SelectedFiles
                    .Select((value, idx) => new { value, idx })
                    .FirstOrDefault(entry => string.Equals(NormalizedPathKey(entry.value), inputKey, StringComparison.OrdinalIgnoreCase))
                    ?.idx;
                if (removeOnException.HasValue)
                {
                    SelectedFiles.RemoveAt(removeOnException.Value);
                }

                if (isForcedPass)
                {
                    RemovePendingFileByKey(inputKey);
                }

                Progress = (processed + 1) / (double)initialTotal;
                await Task.Yield();
                continue;
            }

            if (stepResult.embedError == AwmError.Ok)
            {
                if (stepResult.evidenceError != AwmError.Ok)
                {
                    AddLog(
                        L("证据记录失败", "Evidence record failed"),
                        $"{Path.GetFileName(outputPath)}: {stepResult.evidenceError}",
                        false,
                        true,
                        LogIconTone.Warning);
                }

                successCount += 1;
                string successDetail = $"→ {Path.GetFileName(outputPath)}";
                if (stepResult.snrResult is { } snr && string.Equals(snr.SnrStatus, "ok", StringComparison.OrdinalIgnoreCase) && snr.SnrDb.HasValue)
                {
                    successDetail += $" · SNR {snr.SnrDb.Value:F2} dB";
                }
                else if (stepResult.snrResult is { } nonOkSnr && !string.Equals(nonOkSnr.SnrStatus, "ok", StringComparison.OrdinalIgnoreCase))
                {
                    var isLegacyFfi = string.Equals(nonOkSnr.SnrDetail, "legacy_ffi", StringComparison.OrdinalIgnoreCase);
                    if (!isLegacyFfi || !legacyFfiWarned)
                    {
                        legacyFfiWarned = legacyFfiWarned || isLegacyFfi;
                        var warningDetail = isLegacyFfi
                            ? L("本地核心库版本较旧，暂不支持 SNR 分析", "Native core is outdated and does not support SNR analysis yet")
                            : (nonOkSnr.SnrDetail ?? nonOkSnr.SnrStatus);
                        AddLog(
                            L("SNR 不可用", "SNR unavailable"),
                            warningDetail,
                            false,
                            true,
                            LogIconTone.Warning);
                    }
                }

                AddLog($"{L("成功", "Success")}: {Path.GetFileName(inputPath)}", successDetail, true, false, LogIconTone.Success, LogKind.ResultOk);
            }
            else
            {
                failureCount += 1;
                AddLog($"{L("失败", "Failed")}: {Path.GetFileName(inputPath)}", stepResult.embedError.ToString(), false, false, LogIconTone.Error, LogKind.ResultError);
            }

            var removeIndex = SelectedFiles
                .Select((value, idx) => new { value, idx })
                .FirstOrDefault(entry => string.Equals(NormalizedPathKey(entry.value), inputKey, StringComparison.OrdinalIgnoreCase))
                ?.idx;
            if (removeIndex.HasValue)
            {
                SelectedFiles.RemoveAt(removeIndex.Value);
            }

            if (isForcedPass)
            {
                RemovePendingFileByKey(inputKey);
            }

            Progress = (processed + 1) / (double)initialTotal;
            await Task.Yield();
        }

        if (token.IsCancellationRequested)
        {
            AddLog(L("已取消", "Cancelled"), L($"已完成 {successCount + failureCount} / {initialTotal} 个文件", $"Completed {successCount + failureCount} / {initialTotal} files"), false, false, LogIconTone.Warning);
        }
        else
        {
            AddLog(
                isForcedPass ? L("强行嵌入完成", "Force embed finished") : L("处理完成", "Processing finished"),
                L($"成功: {successCount}, 失败: {failureCount}", $"Success: {successCount}, Failed: {failureCount}"),
                true,
                false,
                LogIconTone.Info);
        }

        if (successCount > 0)
        {
            _ = FlashEmbedSuccessAsync();
            try
            {
                var inserted = await AppViewModel.Instance.TagStore.SaveIfAbsentAsync(username, resolvedTag);
                if (inserted)
                {
                    await RefreshTagMappingsAsync();
                    AddLog(L("已保存映射", "Mapping saved"), $"{username} -> {resolvedTag}", true, false, LogIconTone.Success);
                }
            }
            catch (Exception ex)
            {
                AddLog(
                    L("映射保存失败", "Mapping save failed"),
                    ex.Message,
                    false,
                    true,
                    LogIconTone.Warning);
            }
        }

        CurrentProcessingFile = null;
        CurrentProcessingIndex = -1;
        IsProcessing = false;
        IsCancelling = false;
        ScheduleProgressResetIfNeeded();

        if (!isForcedPass && !token.IsCancellationRequested && deferredFiles.Count > 0)
        {
            PendingForceReviewFiles.Clear();
            foreach (var deferred in deferredFiles)
            {
                PendingForceReviewFiles.Add(deferred);
            }

            AddLog(
                L("发现已含水印文件", "Watermarked files found"),
                L($"共 {deferredFiles.Count} 个文件待确认：可强行嵌入或移出队列", $"{deferredFiles.Count} files require review: force embed or remove from queue"),
                false,
                false,
                LogIconTone.Warning,
                LogKind.ResultNotFound);
            RequestForceReviewPrompt();
        }

        try
        {
            await AppViewModel.Instance.RefreshStatsAsync();
        }
        catch
        {
            // Ignore runtime stats refresh failure.
        }
    }

    private int FindNextQueueIndex(HashSet<string>? targetKeys)
    {
        if (targetKeys is null)
        {
            return SelectedFiles.Count == 0 ? -1 : 0;
        }

        for (var index = 0; index < SelectedFiles.Count; index++)
        {
            if (targetKeys.Contains(NormalizedPathKey(SelectedFiles[index])))
            {
                return index;
            }
        }

        return -1;
    }

    private void RemovePendingFileByKey(string normalizedPathKey)
    {
        for (var index = PendingForceReviewFiles.Count - 1; index >= 0; index--)
        {
            if (string.Equals(
                    NormalizedPathKey(PendingForceReviewFiles[index]),
                    normalizedPathKey,
                    StringComparison.OrdinalIgnoreCase))
            {
                PendingForceReviewFiles.RemoveAt(index);
            }
        }
    }

    private string BuildOutputPath(string inputPath)
    {
        var outputDirectory = !string.IsNullOrWhiteSpace(OutputDirectory)
            ? OutputDirectory!
            : Path.GetDirectoryName(inputPath) ?? throw new InvalidOperationException(L("无法确定输出目录", "Failed to determine output directory"));

        var suffix = string.IsNullOrWhiteSpace(CustomSuffix) ? "_wm" : CustomSuffix.Trim();
        var baseName = Path.GetFileNameWithoutExtension(inputPath);
        var ext = NormalizeOutputExtension(Path.GetExtension(inputPath));
        return Path.Combine(outputDirectory, $"{baseName}{suffix}{ext}");
    }

    private static string NormalizeOutputExtension(string ext)
    {
        return ".wav";
    }

    private IReadOnlyList<string> ResolveAudioFiles(string sourcePath)
    {
        if (Directory.Exists(sourcePath))
        {
            try
            {
                var files = Directory
                    .EnumerateFiles(sourcePath, "*", SearchOption.TopDirectoryOnly)
                    .Where(IsSupportedAudioFile)
                    .ToList();

                if (files.Count == 0)
                {
                    AddLog(
                        L("目录无可用音频", "No audio files in directory"),
                        L("当前目录未找到 WAV / FLAC / M4A / ALAC 文件", "No WAV / FLAC / M4A / ALAC files found in this directory"),
                        false,
                        true,
                        LogIconTone.Warning
                    );
                }

                return files;
            }
            catch (Exception ex)
            {
                AddLog(L("读取目录失败", "Failed to read directory"), ex.Message, false, false, LogIconTone.Error);
                return Array.Empty<string>();
            }
        }

        if (File.Exists(sourcePath) && IsSupportedAudioFile(sourcePath))
        {
            return new[] { sourcePath };
        }

        AddLog(
            L("不支持的输入源", "Unsupported input source"),
            L("请选择 WAV / FLAC / M4A / ALAC 文件或包含这些文件的目录", "Select a WAV / FLAC / M4A / ALAC file or a directory containing those files"),
            false,
            true,
            LogIconTone.Warning
        );
        return Array.Empty<string>();
    }

    private void AppendFilesWithDedup(IEnumerable<string> files)
    {
        var incoming = files
            .Where(path => !string.IsNullOrWhiteSpace(path))
            .ToList();

        if (incoming.Count == 0)
        {
            return;
        }

        var existing = new HashSet<string>(SelectedFiles.Select(NormalizedPathKey), StringComparer.OrdinalIgnoreCase);
        var deduped = new List<string>();
        var duplicateCount = 0;

        foreach (var file in incoming)
        {
            var normalized = NormalizedPathKey(file);
            if (existing.Add(normalized))
            {
                deduped.Add(file);
            }
            else
            {
                duplicateCount += 1;
            }
        }

        foreach (var file in deduped)
        {
            SelectedFiles.Add(file);
        }

        if (duplicateCount > 0)
        {
            AddLog(L("已去重", "Deduplicated"), L($"跳过 {duplicateCount} 个重复文件", $"Skipped {duplicateCount} duplicate files"), true, true, LogIconTone.Info);
        }
    }

    private bool IsSupportedAudioFile(string path)
    {
        var ext = Path.GetExtension(path);
        return _supportedAudioExtensions.Contains(ext);
    }

    private static string NormalizedPathKey(string path)
    {
        try
        {
            return Path.GetFullPath(path).Trim().TrimEnd(Path.DirectorySeparatorChar).ToUpperInvariant();
        }
        catch
        {
            return path.Trim().ToUpperInvariant();
        }
    }

    private static string? NormalizePathOrNull(string? path)
    {
        return string.IsNullOrWhiteSpace(path) ? null : path.Trim();
    }

    private string NormalizedUsernameInput()
    {
        return UsernameInput.Trim();
    }

    private string? ResolveTagValue()
    {
        var username = NormalizedUsernameInput();
        if (string.IsNullOrEmpty(username))
        {
            return null;
        }

        if (MatchedMappingForInput is EmbedMappingOption matched)
        {
            return matched.Tag;
        }

        var identity = SuggestedIdentityFromUsername(username);
        if (string.IsNullOrEmpty(identity))
        {
            return null;
        }

        var (tag, error) = AwmBridge.CreateTag(identity);
        return error == AwmError.Ok ? tag : null;
    }

    private static string SuggestedIdentityFromUsername(string username)
    {
        var digest = SHA256.HashData(Encoding.UTF8.GetBytes(username));

        ulong acc = 0;
        byte accBits = 0;
        var builder = new StringBuilder(7);

        foreach (var b in digest)
        {
            acc = (acc << 8) | b;
            accBits += 8;

            while (accBits >= 5 && builder.Length < 7)
            {
                var shift = accBits - 5;
                var index = (int)((acc >> shift) & 0x1F);
                builder.Append(SuggestedIdentityCharset[index]);
                accBits -= 5;
            }

            if (builder.Length >= 7)
            {
                break;
            }
        }

        return builder.ToString();
    }

    private void UpdateMappingSuggestions()
    {
        var keyword = NormalizedUsernameInput();
        IEnumerable<EmbedMappingOption> query;

        if (string.IsNullOrEmpty(keyword))
        {
            query = AllMappings.OrderBy(x => x.Username, StringComparer.OrdinalIgnoreCase);
        }
        else
        {
            query = AllMappings
                .OrderBy(x => MappingRank(x, keyword))
                .ThenBy(x => x.Username, StringComparer.OrdinalIgnoreCase);
        }

        MappingSuggestions.Clear();
        foreach (var option in query)
        {
            MappingSuggestions.Add(option);
        }

        if (SelectedMapping is not null && !MappingSuggestions.Contains(SelectedMapping))
        {
            _selectedMapping = null;
            OnPropertyChanged(nameof(SelectedMapping));
        }
    }

    private static int MappingRank(EmbedMappingOption option, string keyword)
    {
        if (option.Username.Equals(keyword, StringComparison.OrdinalIgnoreCase))
        {
            return 0;
        }

        if (option.Username.StartsWith(keyword, StringComparison.OrdinalIgnoreCase))
        {
            return 1;
        }

        if (option.Username.Contains(keyword, StringComparison.OrdinalIgnoreCase))
        {
            return 2;
        }

        return 3;
    }

    private void AddLog(
        string title,
        string detail = "",
        bool isSuccess = true,
        bool isEphemeral = false,
        LogIconTone iconTone = LogIconTone.Info,
        LogKind kind = LogKind.Generic)
    {
        var entry = new LogEntry
        {
            Title = title,
            Detail = detail,
            IsSuccess = isSuccess,
            IsEphemeral = isEphemeral,
            IconTone = iconTone,
            Kind = kind,
        };

        Logs.Insert(0, entry);
        while (Logs.Count > MaxLogCount)
        {
            Logs.RemoveAt(Logs.Count - 1);
        }

        if (entry.IsEphemeral && entry.Kind == LogKind.LogsCleared)
        {
            _ = DismissClearLogAsync(entry.Id);
        }
    }

    private async Task DismissClearLogAsync(Guid logId)
    {
        await Task.Delay(TimeSpan.FromSeconds(3));

        var target = Logs.FirstOrDefault(x => x.Id == logId && x.Kind == LogKind.LogsCleared);
        if (target is not null)
        {
            Logs.Remove(target);
        }
    }

    private async Task FlashClearQueueAsync()
    {
        IsClearQueueSuccess = true;
        await Task.Delay(TimeSpan.FromMilliseconds(300));
        IsClearQueueSuccess = false;
    }

    private async Task FlashClearLogsAsync()
    {
        IsClearLogsSuccess = true;
        await Task.Delay(TimeSpan.FromMilliseconds(300));
        IsClearLogsSuccess = false;
    }

    private void ScheduleProgressResetIfNeeded()
    {
        if (Progress < 1)
        {
            return;
        }

        _progressResetCts?.Cancel();
        _progressResetCts = new CancellationTokenSource();
        _ = ResetProgressLaterAsync(_progressResetCts.Token);
    }

    private async Task ResetProgressLaterAsync(CancellationToken token)
    {
        try
        {
            await Task.Delay(TimeSpan.FromSeconds(3), token);
        }
        catch (TaskCanceledException)
        {
            return;
        }

        if (!token.IsCancellationRequested)
        {
            Progress = 0;
        }
    }

    private void OnSelectedFilesChanged(object? sender, NotifyCollectionChangedEventArgs e)
    {
        OnPropertyChanged(nameof(QueueCount));
        OnPropertyChanged(nameof(HasQueueCount));
        OnPropertyChanged(nameof(QueueCountText));
        OnPropertyChanged(nameof(HasQueueFiles));
        OnPropertyChanged(nameof(ShowQueueEmptyHint));
        OnPropertyChanged(nameof(CanEmbedOrStop));
    }

    private void OnLogsChanged(object? sender, NotifyCollectionChangedEventArgs e)
    {
        OnPropertyChanged(nameof(HasLogs));
        OnPropertyChanged(nameof(LogCountText));
        OnPropertyChanged(nameof(ShowNoLogsHint));
    }

    private void OnMappingsChanged(object? sender, NotifyCollectionChangedEventArgs e)
    {
        OnPropertyChanged(nameof(HasMappings));
        OnPropertyChanged(nameof(MappingPlaceholderText));
    }

    private void OnPendingForceReviewFilesChanged(object? sender, NotifyCollectionChangedEventArgs e)
    {
        OnPropertyChanged(nameof(PendingForceReviewCount));
        OnPropertyChanged(nameof(HasPendingForceReview));
        OnPropertyChanged(nameof(ForceReviewDialogMessage));
    }

    private static ObservableCollection<ChannelLayoutOption> BuildChannelLayoutOptions()
    {
        return new ObservableCollection<ChannelLayoutOption>
        {
            CreateLayoutOption(AwmChannelLayout.Auto, L("自动", "Auto")),
            CreateLayoutOption(AwmChannelLayout.Stereo, L("立体声", "Stereo")),
            CreateLayoutOption(AwmChannelLayout.Surround51, "5.1"),
            CreateLayoutOption(AwmChannelLayout.Surround512, "5.1.2"),
            CreateLayoutOption(AwmChannelLayout.Surround71, "7.1"),
            CreateLayoutOption(AwmChannelLayout.Surround714, "7.1.4"),
            CreateLayoutOption(AwmChannelLayout.Surround916, "9.1.6"),
        };
    }

    private void RebuildLayoutOptions()
    {
        var current = SelectedChannelLayout?.Layout ?? AwmChannelLayout.Auto;
        ChannelLayoutOptions.Clear();
        foreach (var option in BuildChannelLayoutOptions())
        {
            ChannelLayoutOptions.Add(option);
        }

        SelectedChannelLayout = ChannelLayoutOptions.FirstOrDefault(item => item.Layout == current)
            ?? ChannelLayoutOptions.FirstOrDefault();
    }

    private static ChannelLayoutOption CreateLayoutOption(AwmChannelLayout layout, string label)
    {
        return new ChannelLayoutOption(layout, label, AwmBridge.GetLayoutChannels(layout));
    }

    public async Task FlashOutputSelectAsync()
    {
        IsOutputSelectSuccess = true;
        await Task.Delay(TimeSpan.FromMilliseconds(900));
        IsOutputSelectSuccess = false;
    }

    private async Task FlashInputSelectAsync()
    {
        IsInputSelectSuccess = true;
        await Task.Delay(TimeSpan.FromMilliseconds(900));
        IsInputSelectSuccess = false;
    }

    private async Task FlashEmbedSuccessAsync()
    {
        IsEmbedSuccess = true;
        await Task.Delay(TimeSpan.FromMilliseconds(900));
        IsEmbedSuccess = false;
    }

    private static Brush ThemePrimaryBrush()
    {
        return ResolveBrush("TextFillColorPrimaryBrush", "NeutralBrush");
    }

    private static Brush ThemeSecondaryBrush()
    {
        return ResolveBrush("TextFillColorSecondaryBrush", "NeutralBrush");
    }

    private static Brush ThemeSuccessBrush()
    {
        return ResolveBrush("SuccessBrush", "TextFillColorPrimaryBrush");
    }

    private static Brush ResolveBrush(string key, string fallbackKey)
    {
        if (Application.Current.Resources.TryGetValue(key, out var brush) && brush is Brush typed)
        {
            return typed;
        }

        if (Application.Current.Resources.TryGetValue(fallbackKey, out var fallback) && fallback is Brush fallbackBrush)
        {
            return fallbackBrush;
        }

        return new SolidColorBrush(Microsoft.UI.Colors.Transparent);
    }

    private void NotifyLocalizedTextChanged()
    {
        OnPropertyChanged(nameof(InputSourceLabel));
        OnPropertyChanged(nameof(OutputDirectoryLabel));
        OnPropertyChanged(nameof(MissingKeyMessage));
        OnPropertyChanged(nameof(GoToKeyPageText));
        OnPropertyChanged(nameof(SelectActionText));
        OnPropertyChanged(nameof(ClearActionText));
        OnPropertyChanged(nameof(SettingsTitle));
        OnPropertyChanged(nameof(UsernameLabel));
        OnPropertyChanged(nameof(UsernamePlaceholder));
        OnPropertyChanged(nameof(StoredMappingsLabel));
        OnPropertyChanged(nameof(StrengthLabel));
        OnPropertyChanged(nameof(SuffixLabel));
        OnPropertyChanged(nameof(LayoutLabel));
        OnPropertyChanged(nameof(DropZoneTitle));
        OnPropertyChanged(nameof(DropZoneSubtitle));
        OnPropertyChanged(nameof(QueueTitle));
        OnPropertyChanged(nameof(QueueEmptyText));
        OnPropertyChanged(nameof(LogsTitle));
        OnPropertyChanged(nameof(LogsEmptyText));
        OnPropertyChanged(nameof(SelectInputSourceAccessibility));
        OnPropertyChanged(nameof(SelectOutputDirectoryAccessibility));
        OnPropertyChanged(nameof(EmbedActionAccessibility));
        OnPropertyChanged(nameof(ClearQueueAccessibility));
        OnPropertyChanged(nameof(ClearLogsAccessibility));
        OnPropertyChanged(nameof(ForceReviewDialogTitle));
        OnPropertyChanged(nameof(ForceReviewDialogPrimaryText));
        OnPropertyChanged(nameof(ForceReviewDialogSecondaryText));
        OnPropertyChanged(nameof(ForceReviewDialogCloseText));
        OnPropertyChanged(nameof(ForceReviewDialogMessage));
    }

    private static string L(string zh, string en) => AppViewModel.Instance.IsEnglishLanguage ? en : zh;

    private static async Task<T> RunCancelableNativeCallAsync<T>(Func<T> operation, CancellationToken token)
    {
        var callTask = Task.Run(operation);
        return await callTask.WaitAsync(token);
    }
}
