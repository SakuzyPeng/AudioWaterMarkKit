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
    public ObservableCollection<string> SkippedWatermarkedFiles { get; } = new();

    private int _skipSummaryPromptVersion;
    public int SkipSummaryPromptVersion
    {
        get => _skipSummaryPromptVersion;
        private set => SetProperty(ref _skipSummaryPromptVersion, value);
    }

    public int SkipSummaryCount => SkippedWatermarkedFiles.Count;
    public bool HasSkippedWatermarkedFiles => SkipSummaryCount > 0;

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
    public string DropZoneSubtitle => _appState.UsingFallbackInputExtensions
        ? L(
            $"支持 {SupportedExtensionsDisplay()}，当前按默认集合处理（运行时能力未知）",
            $"Supports {SupportedExtensionsDisplay()}; using default fallback set while runtime capabilities are unknown")
        : L(
            $"支持 {SupportedExtensionsDisplay()}，可批量拖入",
            $"Supports {SupportedExtensionsDisplay()}, batch drop enabled");
    public string QueueTitle => L("待处理文件", "Pending files");
    public string QueueEmptyText => L("暂无文件", "No files");
    public string LogsTitle => L("事件日志", "Event logs");
    public string LogsEmptyText => L("暂无日志", "No logs");
    public string SelectInputSourceAccessibility => L("选择输入源", "Select input source");
    public string SelectOutputDirectoryAccessibility => L("选择输出目录", "Select output directory");
    public string ClearInputSourceAccessibility => L("清空输入源地址", "Clear input source path");
    public string ClearOutputDirectoryAccessibility => L("清空输出目录地址", "Clear output directory path");
    public string EmbedActionAccessibility => L("开始或停止嵌入", "Start or stop embedding");
    public string ClearQueueAccessibility => L("清空队列", "Clear queue");
    public string ClearLogsAccessibility => L("清空日志", "Clear logs");

    public string PreviewTagText => ResolveTagValue() ?? "-";
    public string PreviewTagDisplay => $"Tag: {PreviewTagText}";
    public string? MatchedMappingHintText => MatchedMappingForInput is null ? null : L("已存在映射，自动复用", "Existing mapping found, auto reused");
    public string ReuseHintText => MatchedMappingForInput is null ? string.Empty : L("复用 ", "Reusing ");
    public string MappingPlaceholderText => HasMappings ? L("选择已存储映射", "Select stored mapping") : L("暂无已存储映射", "No stored mappings");
    public string SkipSummaryDialogTitle => L("已跳过含水印文件", "Skipped watermarked files");
    public string SkipSummaryDialogCloseText => L("我知道了", "OK");
    public string SkipSummaryDialogMessage
    {
        get
        {
            if (SkipSummaryCount == 0)
            {
                return L("未检测到已含水印文件。", "No already-watermarked files were detected.");
            }

            var preview = string.Join("、", SkippedWatermarkedFiles.Take(3).Select(Path.GetFileName));
            if (SkipSummaryCount <= 3)
            {
                return L(
                    $"已跳过 {SkipSummaryCount} 个已含水印文件：{preview}\n该类文件已自动跳过。",
                    $"Skipped {SkipSummaryCount} already-watermarked files: {preview}\nThese files were skipped automatically.");
            }

            var remain = SkipSummaryCount - 3;
            return L(
                $"已跳过 {SkipSummaryCount} 个已含水印文件：{preview} 等 {remain} 个\n该类文件已自动跳过。",
                $"Skipped {SkipSummaryCount} already-watermarked files: {preview} and {remain} more\nThese files were skipped automatically.");
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
        SkippedWatermarkedFiles.CollectionChanged += OnSkippedWatermarkedFilesChanged;
        _appState.PropertyChanged += OnAppStatePropertyChanged;
        _ = RefreshTagMappingsAsync();
    }

    private void OnAppStatePropertyChanged(object? sender, PropertyChangedEventArgs e)
    {
        if (e.PropertyName == nameof(AppViewModel.UiLanguageCode))
        {
            OnPropertyChanged(nameof(InputSourceText));
            OnPropertyChanged(nameof(OutputDirectoryText));
            OnPropertyChanged(nameof(QueueCountText));
            OnPropertyChanged(nameof(LogCountText));
            OnPropertyChanged(nameof(EmbedButtonText));
            OnPropertyChanged(nameof(MatchedMappingHintText));
            OnPropertyChanged(nameof(ReuseHintText));
            OnPropertyChanged(nameof(MappingPlaceholderText));
            OnPropertyChanged(nameof(SkipSummaryDialogTitle));
            OnPropertyChanged(nameof(SkipSummaryDialogCloseText));
            OnPropertyChanged(nameof(SkipSummaryDialogMessage));
            NotifyLocalizedTextChanged();
            RebuildLayoutOptions();
            return;
        }

        if (e.PropertyName == nameof(AppViewModel.EngineCapsKnown)
            || e.PropertyName == nameof(AppViewModel.EngineContainerMp4)
            || e.PropertyName == nameof(AppViewModel.EngineContainerMkv)
            || e.PropertyName == nameof(AppViewModel.EngineContainerTs)
            || e.PropertyName == nameof(AppViewModel.EffectiveSupportedInputExtensionsDisplay))
        {
            OnPropertyChanged(nameof(DropZoneSubtitle));
            return;
        }
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
        var unsupported = new List<string>();
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
            else if (File.Exists(path))
            {
                unsupported.Add(path);
            }
        }

        LogUnsupportedDroppedFiles(unsupported);
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

        SelectedFiles.Remove(filePath);
    }

    [RelayCommand]
    private void ClearInputSource()
    {
        if (string.IsNullOrWhiteSpace(InputSource))
        {
            AddLog(
                L("输入源为空", "Input source is empty"),
                L("没有可清空的输入源地址", "No input source path to clear"),
                true,
                true,
                LogIconTone.Info);
            return;
        }

        InputSource = null;
        AddLog(
            L("已清空输入源", "Input source cleared"),
            L("仅清空输入源地址，不影响待处理队列", "Cleared input source path only; queue unchanged"),
            true,
            true,
            LogIconTone.Info);
    }

    [RelayCommand]
    private void ClearOutputDirectory()
    {
        if (string.IsNullOrWhiteSpace(OutputDirectory))
        {
            AddLog(
                L("输出目录为空", "Output directory is empty"),
                L("没有可清空的输出目录地址", "No output directory path to clear"),
                true,
                true,
                LogIconTone.Info);
            return;
        }

        OutputDirectory = null;
        AddLog(
            L("已清空输出目录", "Output directory cleared"),
            L("已恢复为写回源文件目录", "Reset to write-back source directory"),
            true,
            true,
            LogIconTone.Info);
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
        SkippedWatermarkedFiles.Clear();
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

    private void RequestSkipSummaryPrompt()
    {
        SkipSummaryPromptVersion += 1;
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

    private async Task EmbedAsync()
    {
        if (IsProcessing)
        {
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
            AddLog(L("嵌入失败", "Embed failed"), $"{L("密钥不可用", "Key unavailable")}: {DescribeAwmError(keyError)}", false, false, LogIconTone.Error);
            return;
        }

        var (message, encodeError) = AwmBridge.EncodeMessage(resolvedTag, key, _appState.ActiveKeySlot);
        if (message is null || encodeError != AwmError.Ok)
        {
            AddLog(L("嵌入失败", "Embed failed"), $"{L("消息编码失败", "Message encode failed")}: {DescribeAwmError(encodeError)}", false, false, LogIconTone.Error);
            return;
        }

        _progressResetCts?.Cancel();
        _embedCts = new CancellationTokenSource();
        var token = _embedCts.Token;

        IsProcessing = true;
        IsCancelling = false;
        Progress = 0;
        CurrentProcessingIndex = 0;
        SkippedWatermarkedFiles.Clear();

        var layout = SelectedChannelLayout?.Layout ?? AwmChannelLayout.Auto;
        var layoutText = SelectedChannelLayout?.DisplayText ?? L("自动", "Auto");
        AddLog(
            L("开始处理", "Processing started"),
            L($"准备处理 {SelectedFiles.Count} 个文件（{layoutText}）", $"Preparing to process {SelectedFiles.Count} files ({layoutText})"),
            true,
            false,
            LogIconTone.Info);

        var initialTotal = Math.Max(SelectedFiles.Count, 1);
        var successCount = 0;
        var failureCount = 0;
        var skippedFiles = new List<string>();
        var skippedKeys = new HashSet<string>(StringComparer.OrdinalIgnoreCase);
        var legacyFfiWarned = false;
        var admPrecheckWarned = false;

        for (var processed = 0; processed < initialTotal; processed++)
        {
            if (token.IsCancellationRequested)
            {
                break;
            }

            var queueIndex = SelectedFiles.Count == 0 ? -1 : 0;
            if (queueIndex < 0 || queueIndex >= SelectedFiles.Count)
            {
                break;
            }

            var inputPath = SelectedFiles[queueIndex];
            var inputKey = NormalizedPathKey(inputPath);
            CurrentProcessingFile = Path.GetFileName(inputPath);
            CurrentProcessingIndex = queueIndex;

            (bool detected, AwmError error, bool admPrecheckSkipped) precheckResult;
            try
            {
                precheckResult = await RunCancelableNativeCallAsync(() =>
                {
                    var detect = AwmBridge.DetectAudioMultichannelDetailed(inputPath, layout);
                    if (detect.error == AwmError.Ok && detect.result is not null)
                    {
                        return (true, AwmError.Ok, false);
                    }

                    if (detect.error == AwmError.NoWatermarkFound)
                    {
                        return (false, AwmError.Ok, false);
                    }

                    if (detect.error == AwmError.AdmUnsupported)
                    {
                        return (false, AwmError.Ok, true);
                    }

                    return (false, detect.error, false);
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
                    $"{L("预检失败", "Precheck failed")}: {DescribeAwmError(precheckResult.error)}",
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

            if (precheckResult.admPrecheckSkipped && !admPrecheckWarned)
            {
                admPrecheckWarned = true;
                AddLog(
                    L("预检已跳过", "Precheck skipped"),
                    L(
                        "ADM/BWF 检测暂不支持，已跳过预检并继续嵌入",
                        "ADM/BWF detect is not supported yet; precheck was skipped and embed continues"
                    ),
                    false,
                    true,
                    LogIconTone.Warning);
            }

            if (precheckResult.detected)
            {
                if (queueIndex < SelectedFiles.Count)
                {
                    var skipped = SelectedFiles[queueIndex];
                    SelectedFiles.RemoveAt(queueIndex);
                    if (skippedKeys.Add(inputKey))
                    {
                        skippedFiles.Add(skipped);
                    }
                }

                AddLog(
                    L("检测到已有水印", "Existing watermark detected"),
                    L($"{Path.GetFileName(inputPath)} 已跳过", $"{Path.GetFileName(inputPath)} skipped"),
                    false,
                    false,
                    LogIconTone.Warning,
                    LogKind.ResultNotFound);
                Progress = (processed + 1) / (double)initialTotal;
                await Task.Yield();
                continue;
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
                            false);
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
                        $"{Path.GetFileName(outputPath)}: {DescribeAwmError(stepResult.evidenceError)}",
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
                AddLog(
                    $"{L("失败", "Failed")}: {Path.GetFileName(inputPath)}",
                    DescribeAwmError(stepResult.embedError),
                    false,
                    false,
                    LogIconTone.Error,
                    LogKind.ResultError);
            }

            var removeIndex = SelectedFiles
                .Select((value, idx) => new { value, idx })
                .FirstOrDefault(entry => string.Equals(NormalizedPathKey(entry.value), inputKey, StringComparison.OrdinalIgnoreCase))
                ?.idx;
            if (removeIndex.HasValue)
            {
                SelectedFiles.RemoveAt(removeIndex.Value);
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
                L("处理完成", "Processing finished"),
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

        if (!token.IsCancellationRequested && skippedFiles.Count > 0)
        {
            SkippedWatermarkedFiles.Clear();
            foreach (var skipped in skippedFiles)
            {
                SkippedWatermarkedFiles.Add(skipped);
            }

            AddLog(
                L("已跳过含水印文件", "Skipped watermarked files"),
                L($"共跳过 {skippedFiles.Count} 个已含水印文件", $"Skipped {skippedFiles.Count} already-watermarked files"),
                false,
                false,
                LogIconTone.Warning,
                LogKind.ResultNotFound);
            RequestSkipSummaryPrompt();
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
                    .ToList();
                var supported = files
                    .Where(IsSupportedAudioFile)
                    .ToList();
                var unsupported = files
                    .Where(path => !IsSupportedAudioFile(path))
                    .ToList();
                LogUnsupportedDroppedFiles(unsupported);

                if (supported.Count == 0)
                {
                    AddLog(
                        L("目录无可用音频", "No audio files in directory"),
                        BuildDirectoryNoAudioDetail(),
                        false,
                        true,
                        LogIconTone.Warning
                    );
                }

                return supported;
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
            BuildUnsupportedInputDetail(),
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
        return EffectiveSupportedAudioExtensions().Contains(ext, StringComparer.OrdinalIgnoreCase);
    }

    private IReadOnlyList<string> EffectiveSupportedAudioExtensions()
    {
        return _appState.EffectiveSupportedInputExtensions();
    }

    private string SupportedExtensionsDisplay()
    {
        return _appState.EffectiveSupportedInputExtensionsDisplay;
    }

    private string BuildDirectoryNoAudioDetail()
    {
        var extText = SupportedExtensionsDisplay();
        return _appState.UsingFallbackInputExtensions
            ? L(
                $"当前目录未找到 {extText} 文件（按默认支持集合处理）",
                $"No {extText} files found in this directory (fallback set applied)")
            : L(
                $"当前目录未找到 {extText} 文件",
                $"No {extText} files found in this directory");
    }

    private string BuildUnsupportedInputDetail()
    {
        var extText = SupportedExtensionsDisplay();
        return _appState.UsingFallbackInputExtensions
            ? L(
                $"请选择 {extText} 文件或包含这些文件的目录。当前按默认集合处理，运行时缺少 demuxer 时仍可能失败",
                $"Select a {extText} file or directory containing these files. Using fallback set now; execution can still fail if demuxers are missing")
            : L(
                $"请选择 {extText} 文件或包含这些文件的目录",
                $"Select a {extText} file or directory containing these files");
    }

    private void LogUnsupportedDroppedFiles(IReadOnlyList<string> unsupported)
    {
        if (unsupported.Count == 0)
        {
            return;
        }

        var unique = unsupported
            .Select(NormalizedPathKey)
            .Distinct(StringComparer.OrdinalIgnoreCase)
            .ToList();
        if (unique.Count == 0)
        {
            return;
        }

        var preview = string.Join(", ", unique.Take(3).Select(Path.GetFileName));
        var remain = unique.Count - Math.Min(unique.Count, 3);
        var detail = remain <= 0
            ? L($"已跳过 {unique.Count} 个不支持文件：{preview}", $"Skipped {unique.Count} unsupported file(s): {preview}")
            : L($"已跳过 {unique.Count} 个不支持文件：{preview} 等 {remain} 个", $"Skipped {unique.Count} unsupported file(s): {preview} and {remain} more");

        AddLog(
            L("已跳过不支持文件", "Skipped unsupported files"),
            detail,
            false,
            true,
            LogIconTone.Warning
        );
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

    private void OnSkippedWatermarkedFilesChanged(object? sender, NotifyCollectionChangedEventArgs e)
    {
        OnPropertyChanged(nameof(SkipSummaryCount));
        OnPropertyChanged(nameof(HasSkippedWatermarkedFiles));
        OnPropertyChanged(nameof(SkipSummaryDialogMessage));
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
        OnPropertyChanged(nameof(ClearInputSourceAccessibility));
        OnPropertyChanged(nameof(ClearOutputDirectoryAccessibility));
        OnPropertyChanged(nameof(EmbedActionAccessibility));
        OnPropertyChanged(nameof(ClearQueueAccessibility));
        OnPropertyChanged(nameof(ClearLogsAccessibility));
        OnPropertyChanged(nameof(SkipSummaryDialogTitle));
        OnPropertyChanged(nameof(SkipSummaryDialogCloseText));
        OnPropertyChanged(nameof(SkipSummaryDialogMessage));
    }

    private static string L(string zh, string en) => AppViewModel.Instance.IsEnglishLanguage ? en : zh;

    private static string DescribeAwmError(AwmError error)
    {
        return error switch
        {
            AwmError.InvalidOutputFormat => L(
                "输出格式无效：仅支持 .wav",
                "Invalid output format: output must be .wav"
            ),
            AwmError.AdmUnsupported => L(
                "ADM/BWF 不支持：当前操作或元数据结构不受支持",
                "ADM/BWF unsupported for current operation or metadata layout"
            ),
            AwmError.AdmPreserveFailed => L(
                "ADM/BWF 元数据保真失败：为避免破坏母版已中止输出",
                "Failed to preserve ADM/BWF metadata; output was aborted to protect the master"
            ),
            AwmError.AdmPcmFormatUnsupported => L(
                "ADM/BWF PCM 格式不支持：仅支持 16/24/32-bit PCM",
                "Unsupported ADM/BWF PCM format: only 16/24/32-bit PCM"
            ),
            _ => error.ToString(),
        };
    }

    private static async Task<T> RunCancelableNativeCallAsync<T>(Func<T> operation, CancellationToken token)
    {
        var callTask = Task.Run(operation);
        return await callTask.WaitAsync(token);
    }
}
