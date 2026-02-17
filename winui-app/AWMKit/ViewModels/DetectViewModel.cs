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
using System.Threading;
using System.Threading.Tasks;

namespace AWMKit.ViewModels;

/// <summary>
/// Detect page state model aligned with macOS behavior.
/// </summary>
public sealed partial class DetectViewModel : ObservableObject
{
    private const int MaxLogCount = 200;

    private static Brush SuccessBrush => ThemeBrush("SuccessBrush", "TextFillColorPrimaryBrush");
    private static Brush WarningBrush => ThemeBrush("WarningBrush", "TextFillColorPrimaryBrush");
    private static Brush ErrorColorBrush => ThemeBrush("ErrorBrush", "TextFillColorPrimaryBrush");
    private static Brush InfoBrush => ThemeBrush("InfoBrush", "TextFillColorPrimaryBrush");
    private static Brush SecondaryBrush => ThemeBrush("TextFillColorSecondaryBrush", "NeutralBrush");
    private static Brush PrimaryBrush => ThemeBrush("TextFillColorPrimaryBrush", "NeutralBrush");
    private static Brush YellowBrush => ThemeBrush("YellowBrush", "WarningBrush");

    private CancellationTokenSource? _detectCts;
    private CancellationTokenSource? _progressResetCts;
    private readonly AppViewModel _appState = AppViewModel.Instance;

    private bool _isInputSelectSuccess;
    public bool IsInputSelectSuccess
    {
        get => _isInputSelectSuccess;
        private set => SetProperty(ref _isInputSelectSuccess, value);
    }

    private bool _isDetectSuccess;
    public bool IsDetectSuccess
    {
        get => _isDetectSuccess;
        private set
        {
            if (SetProperty(ref _isDetectSuccess, value))
            {
                OnPropertyChanged(nameof(ShowDetectDefaultPlayIcon));
                OnPropertyChanged(nameof(ShowDetectSuccessPlayIcon));
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
                OnPropertyChanged(nameof(CanDetectOrStop));
                OnPropertyChanged(nameof(DetectButtonText));
                OnPropertyChanged(nameof(ShowDetectStopIcon));
                OnPropertyChanged(nameof(ShowDetectDefaultPlayIcon));
                OnPropertyChanged(nameof(ShowDetectSuccessPlayIcon));
            }
        }
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

    private int _totalDetected;
    public int TotalDetected
    {
        get => _totalDetected;
        private set
        {
            if (SetProperty(ref _totalDetected, value))
            {
                OnPropertyChanged(nameof(HasDetectCount));
                OnPropertyChanged(nameof(DetectCountText));
            }
        }
    }

    private int _totalFound;
    public int TotalFound
    {
        get => _totalFound;
        private set
        {
            if (SetProperty(ref _totalFound, value))
            {
                OnPropertyChanged(nameof(DetectCountText));
            }
        }
    }

    public bool HasDetectCount => TotalDetected > 0;

    public string DetectCountText => $"{TotalFound}{L("（成功）", " (success)")}/{TotalDetected}{L("（总）", " (total)")}";

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

    private string _logSearchText = string.Empty;
    public string LogSearchText
    {
        get => _logSearchText;
        set
        {
            if (SetProperty(ref _logSearchText, value))
            {
                NotifyFilteredLogsChanged();
            }
        }
    }

    private Guid? _selectedResultLogId;
    public Guid? SelectedResultLogId
    {
        get => _selectedResultLogId;
        private set
        {
            if (SetProperty(ref _selectedResultLogId, value))
            {
                RefreshLogSelectionState();
                NotifyDisplayedRecordChanged();
            }
        }
    }

    private bool _hideDetectDetailWhenNoSelection;
    public bool HideDetectDetailWhenNoSelection
    {
        get => _hideDetectDetailWhenNoSelection;
        private set
        {
            if (SetProperty(ref _hideDetectDetailWhenNoSelection, value))
            {
                NotifyDisplayedRecordChanged();
            }
        }
    }

    public ObservableCollection<string> SelectedFiles { get; } = new();

    public ObservableCollection<LogEntry> Logs { get; } = new();

    public ObservableCollection<DetectRecord> DetectRecords { get; } = new();

    public bool HasLogs => Logs.Count > 0;

    public bool ShowSearchBox => HasLogs;

    public bool ShowNoLogsHint => !HasLogs;

    public string LogCountText => L($"共 {Logs.Count} 条", $"Total {Logs.Count}");

    public bool HasFilteredLogs => FilteredLogs.Any();

    public bool ShowNoFilteredLogsHint => HasLogs && !HasFilteredLogs;

    public IEnumerable<LogEntry> FilteredLogs
    {
        get
        {
            var query = LogSearchText.Trim();
            if (string.IsNullOrEmpty(query))
            {
                return Logs;
            }

            return Logs.Where(log =>
                log.Title.Contains(query, StringComparison.OrdinalIgnoreCase) ||
                log.Detail.Contains(query, StringComparison.OrdinalIgnoreCase));
        }
    }

    public int QueueCount => SelectedFiles.Count;

    public bool HasQueueCount => QueueCount > 0;

    public bool HasQueueFiles => SelectedFiles.Count > 0;

    public bool ShowQueueEmptyHint => !HasQueueFiles;

    public string QueueCountText => L($"共 {QueueCount} 个", $"Total {QueueCount}");

    private bool _isKeyAvailable;
    public bool IsKeyAvailable
    {
        get => _isKeyAvailable;
        set
        {
            if (SetProperty(ref _isKeyAvailable, value))
            {
                OnPropertyChanged(nameof(CanDetectOrStop));
            }
        }
    }

    public bool CanDetectOrStop => IsProcessing || SelectedFiles.Count > 0;

    public string DetectButtonText => IsProcessing ? L("停止", "Stop") : L("检测", "Detect");
    public bool ShowDetectStopIcon => IsProcessing;
    public bool ShowDetectDefaultPlayIcon => !IsProcessing && !IsDetectSuccess;
    public bool ShowDetectSuccessPlayIcon => !IsProcessing && IsDetectSuccess;
    public string InputSectionTitle => L("待检测文件", "Input files");
    public string MissingKeyMessage => L("未配置密钥，请前往密钥页完成生成。", "No key configured. Please go to Key page to create one.");
    public string GoToKeyPageText => L("前往密钥页", "Go to Key page");
    public string SelectActionText => L("选择", "Select");
    public string ClearActionText => L("清空", "Clear");
    public string LayoutLabel => L("声道布局", "Channel layout");
    public string DropZoneTitle => L("拖拽音频文件到此处", "Drag audio files here");
    public string DropZoneSubtitle => _appState.UsingFallbackInputExtensions
        ? L(
            $"支持 {SupportedExtensionsDisplay()}，当前按默认集合处理（运行时能力未知）",
            $"Supports {SupportedExtensionsDisplay()}; using default fallback set while runtime capabilities are unknown")
        : L(
            $"支持 {SupportedExtensionsDisplay()}，可批量拖入并检测",
            $"Supports {SupportedExtensionsDisplay()}, batch drop enabled for detection");
    public string DetectInfoTitle => L("检测信息", "Detection info");
    public string FileFieldLabel => L("文件", "File");
    public string StatusFieldLabel => L("状态", "Status");
    public string MatchFieldLabel => L("匹配标记", "Match");
    public string PatternFieldLabel => L("检测模式", "Mode");
    public string TagFieldLabel => L("标签", "Tag");
    public string IdentityFieldLabel => L("身份", "Identity");
    public string VersionFieldLabel => L("版本", "Version");
    public string TimeFieldLabel => L("检测时间", "Detect time");
    public string KeySlotFieldLabel => L("密钥槽位", "Key slot");
    public string BitErrorsFieldLabel => L("位错误", "Bit errors");
    public string DetectScoreFieldLabel => L("检测分数", "Detect score");
    public string CloneCheckFieldLabel => L("克隆校验", "Clone check");
    public string FingerprintScoreFieldLabel => L("指纹分数", "Fingerprint score");
    public string ErrorFieldLabel => L("错误信息", "Error");
    public string QueueTitle => L("待检测文件", "Pending files");
    public string QueueEmptyText => L("暂无文件", "No files");
    public string LogsTitle => L("检测日志", "Detection logs");
    public string LogSearchPlaceholder => L("搜索日志（标题/详情）", "Search logs (title/detail)");
    public string NoFilteredLogsText => L("暂无或无匹配日志", "No logs or no matches");
    public string SelectInputSourceAccessibility => L("选择输入源", "Select input source");
    public string ClearInputSourceAccessibility => L("清空输入源地址", "Clear input source path");
    public string DetectActionAccessibility => L("开始或停止检测", "Start or stop detection");
    public string ClearQueueAccessibility => L("清空队列", "Clear queue");
    public string ClearLogsAccessibility => L("清空日志", "Clear logs");

    public DetectRecord? DisplayedRecord
    {
        get
        {
            if (SelectedResultLogId.HasValue)
            {
                var selectedLog = Logs.FirstOrDefault(x => x.Id == SelectedResultLogId.Value);
                if (selectedLog?.RelatedRecordId is Guid relatedId)
                {
                    var record = DetectRecords.FirstOrDefault(x => x.Id == relatedId);
                    if (record is not null)
                    {
                        return record;
                    }
                }
            }

            if (HideDetectDetailWhenNoSelection)
            {
                return null;
            }

            return DetectRecords.FirstOrDefault();
        }
    }

    public string DisplayFile => DetailValue(DisplayedRecord?.FilePath);

    public string DisplayStatus => StatusDisplayValue(DisplayedRecord?.Status);

    public string DisplayMatchFound => DisplayedRecord?.MatchFound switch
    {
        true => L("是", "true"),
        false => L("否", "false"),
        _ => "-",
    };

    public string DisplayPattern => DetailValue(DisplayedRecord?.Pattern);

    public string DisplayTag => DetailValue(DisplayedRecord?.Tag);

    public string DisplayIdentity => DetailValue(DisplayedRecord?.Identity);

    public string DisplayVersion => DisplayedRecord?.Version is byte version ? version.ToString() : "-";

    public string DisplayDetectTime => LocalTimestampDisplay(DisplayedRecord);

    public string DisplayKeySlot => DisplayedRecord?.KeySlot is byte keySlot ? keySlot.ToString() : "-";

    public string DisplayBitErrors => DisplayedRecord?.BitErrors is uint bitErrors ? bitErrors.ToString() : "-";

    public string DisplayDetectScore => DisplayedRecord?.DetectScore is float score ? $"{score:0.000}" : "-";

    public string DisplayCloneCheck => CloneCheckDisplayValue(DisplayedRecord?.CloneCheck);

    public string DisplayFingerprintScore => FingerprintScoreDisplay(DisplayedRecord);

    public string DisplayError => ErrorDisplayValue(DisplayedRecord);

    public Brush StatusBrush => FieldBrush(FieldSemantic.Status, DisplayedRecord);

    public Brush MatchFoundBrush => FieldBrush(FieldSemantic.MatchFound, DisplayedRecord);

    public Brush BitErrorsBrush => FieldBrush(FieldSemantic.BitErrors, DisplayedRecord);

    public Brush DetectScoreBrush => FieldBrush(FieldSemantic.DetectScore, DisplayedRecord);

    public Brush CloneCheckBrush => FieldBrush(FieldSemantic.CloneCheck, DisplayedRecord);

    public Brush FingerprintScoreBrush => FieldBrush(FieldSemantic.FingerprintScore, DisplayedRecord);

    public Brush ErrorBrush => FieldBrush(FieldSemantic.Error, DisplayedRecord);

    public DetectViewModel()
    {
        SelectedChannelLayout = ChannelLayoutOptions.FirstOrDefault();
        SelectedFiles.CollectionChanged += OnSelectedFilesChanged;
        Logs.CollectionChanged += OnLogsChanged;
        DetectRecords.CollectionChanged += OnDetectRecordsChanged;
        _appState.PropertyChanged += OnAppStatePropertyChanged;
    }

    private void OnAppStatePropertyChanged(object? sender, PropertyChangedEventArgs e)
    {
        if (e.PropertyName == nameof(AppViewModel.UiLanguageCode))
        {
            OnPropertyChanged(nameof(InputSourceText));
            OnPropertyChanged(nameof(DetectCountText));
            OnPropertyChanged(nameof(LogCountText));
            OnPropertyChanged(nameof(QueueCountText));
            OnPropertyChanged(nameof(DetectButtonText));
            OnPropertyChanged(nameof(DisplayStatus));
            OnPropertyChanged(nameof(DisplayMatchFound));
            OnPropertyChanged(nameof(DisplayPattern));
            OnPropertyChanged(nameof(DisplayDetectTime));
            OnPropertyChanged(nameof(DisplayCloneCheck));
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
        var files = new List<string>();
        var unsupported = new List<string>();
        foreach (var path in filePaths)
        {
            if (Directory.Exists(path))
            {
                files.AddRange(ResolveAudioFiles(path));
            }
            else if (IsSupportedAudioFile(path))
            {
                files.Add(path);
            }
            else if (File.Exists(path))
            {
                unsupported.Add(path);
            }
        }

        LogUnsupportedDroppedFiles(unsupported);
        AppendFilesWithDedup(files);
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
                null,
                LogIconTone.Info);
            return;
        }

        InputSource = null;
        AddLog(
            L("已清空输入源", "Input source cleared"),
            L("仅清空输入源地址，不影响待处理队列", "Cleared input source path only; queue unchanged"),
            true,
            true,
            null,
            LogIconTone.Info);
    }

    [RelayCommand]
    private async Task ClearQueueAsync()
    {
        if (!SelectedFiles.Any())
        {
            AddLog(L("队列为空", "Queue is empty"), L("没有可移除的文件", "No files to remove"), true, true, null, LogIconTone.Info);
            return;
        }

        var count = SelectedFiles.Count;
        SelectedFiles.Clear();
        AddLog(L("已清空队列", "Queue cleared"), L($"移除了 {count} 个文件", $"Removed {count} files"), true, false, null, LogIconTone.Success, LogKind.QueueCleared);
        await FlashClearQueueAsync();
    }

    [RelayCommand]
    private async Task ClearLogsAsync()
    {
        if (!Logs.Any())
        {
            AddLog(L("日志为空", "Logs are empty"), L("没有可清空的日志", "No logs to clear"), true, true, null, LogIconTone.Info);
            return;
        }

        var count = Logs.Count;
        Logs.Clear();
        DetectRecords.Clear();
        TotalDetected = 0;
        TotalFound = 0;

        AddLog(L("已清空日志", "Logs cleared"), L($"移除了 {count} 条日志记录", $"Removed {count} log entries"), true, true, null, LogIconTone.Success, LogKind.LogsCleared);
        await FlashClearLogsAsync();
    }

    [RelayCommand]
    private void ToggleLogSelection(LogEntry? entry)
    {
        if (entry is null || !entry.IsSelectable)
        {
            return;
        }

        if (SelectedResultLogId == entry.Id)
        {
            SelectedResultLogId = null;
            HideDetectDetailWhenNoSelection = true;
            return;
        }

        SelectedResultLogId = entry.Id;
        HideDetectDetailWhenNoSelection = false;
    }

    [RelayCommand]
    private async Task DetectOrStopAsync()
    {
        try
        {
            if (IsProcessing)
            {
                _detectCts?.Cancel();
                AddLog(L("检测已停止", "Detection stopped"), L("用户手动停止", "Stopped by user"), false, true, null, LogIconTone.Warning);
                return;
            }

            await DetectAsync();
        }
        catch (Exception ex)
        {
            AddLog(L("检测异常", "Detection error"), ex.Message, false, false, null, LogIconTone.Error);
            CurrentProcessingFile = null;
            CurrentProcessingIndex = -1;
            IsProcessing = false;
        }
    }

    private async Task DetectAsync()
    {
        if (IsProcessing)
        {
            return;
        }

        if (!SelectedFiles.Any())
        {
            AddLog(L("队列为空", "Queue is empty"), L("请先添加音频文件", "Add audio files first"), false, true, null, LogIconTone.Warning);
            return;
        }

        try
        {
            byte[]? key = null;
            var (loadedKey, keyError) = AwmKeyBridge.LoadKey();
            if (keyError == AwmError.Ok && loadedKey is not null)
            {
                key = loadedKey;
            }
            else
            {
                AddLog(
                    L("未配置密钥", "Key not configured"),
                    L(
                        "将仅显示未校验结果，且不可用于归属/取证",
                        "Only unverified fields will be shown. Do not use for attribution/forensics"
                    ),
                    false,
                    true,
                    null,
                    LogIconTone.Warning);
            }

            _detectCts = new CancellationTokenSource();
            var token = _detectCts.Token;

            _progressResetCts?.Cancel();
            IsProcessing = true;
            Progress = 0;
            CurrentProcessingIndex = 0;
            TotalDetected = 0;
            TotalFound = 0;
            var channelLayout = SelectedChannelLayout?.Layout ?? AwmChannelLayout.Auto;
            var layoutText = SelectedChannelLayout?.DisplayText ?? L("自动", "Auto");

            AddLog(
                L("开始检测", "Detection started"),
                L($"准备检测 {SelectedFiles.Count} 个文件（{layoutText}）", $"Preparing to detect {SelectedFiles.Count} files ({layoutText})"),
                true,
                false,
                null,
                LogIconTone.Info);

            var initialQueue = SelectedFiles.ToList();
            var initialTotal = Math.Max(initialQueue.Count, 1);
            var weightByFile = BuildProgressWeights(initialQueue);
            var totalWeight = Math.Max(weightByFile.Values.Sum(), 1.0);
            var doneWeight = 0.0;
            var uiContext = SynchronizationContext.Current;
            for (var processed = 0; processed < initialTotal; processed++)
            {
                if (token.IsCancellationRequested || SelectedFiles.Count == 0)
                {
                    break;
                }

                var filePath = SelectedFiles[0];
                var fileKey = NormalizedPathKey(filePath);
                var fileWeight = weightByFile.TryGetValue(fileKey, out var resolvedWeight) ? resolvedWeight : 1.0;
                var fileProgress = 0.0;
                void UpdateFileProgress(double candidate)
                {
                    var clamped = Math.Clamp(candidate, 0, 1);
                    if (clamped <= fileProgress)
                    {
                        return;
                    }

                    fileProgress = clamped;
                    Progress = Math.Min(1, (doneWeight + (fileWeight * fileProgress)) / totalWeight);
                }

                void DispatchFileProgress(double candidate)
                {
                    if (uiContext is null)
                    {
                        UpdateFileProgress(candidate);
                        return;
                    }

                    uiContext.Post(_ => UpdateFileProgress(candidate), null);
                }

                var fileName = Path.GetFileName(filePath);
                CurrentProcessingFile = fileName;
                CurrentProcessingIndex = 0;
                UpdateFileProgress(0.02);

                DetectRecord record;
                try
                {
                    var progressLock = new object();
                    AwmProgressPhase lastPhase = AwmProgressPhase.Idle;
                    var callbackProgress = fileProgress;
                    record = await Task.Run(() => DetectSingleFile(
                        filePath,
                        key,
                        channelLayout,
                        snapshot =>
                        {
                            if (snapshot.Operation != AwmProgressOperation.Detect)
                            {
                                return;
                            }

                            double nextProgress;
                            lock (progressLock)
                            {
                                nextProgress = MapSnapshotProgress(
                                    snapshot,
                                    ProgressProfile.Detect,
                                    ref lastPhase,
                                    callbackProgress);
                                if (nextProgress <= callbackProgress)
                                {
                                    return;
                                }
                                callbackProgress = nextProgress;
                            }

                            DispatchFileProgress(nextProgress);
                        }), token);
                }
                catch (Exception ex)
                {
                    record = new DetectRecord
                    {
                        FilePath = filePath,
                        Status = "error",
                        Error = ex.Message,
                    };
                }

                if (token.IsCancellationRequested)
                {
                    break;
                }

                InsertDetectRecord(record);
                LogDetectionOutcome(fileName, record);

                TotalDetected += 1;
                if (record.Status == "ok")
                {
                    TotalFound += 1;
                }

                if (SelectedFiles.Count > 0)
                {
                    SelectedFiles.RemoveAt(0);
                }

                doneWeight += fileWeight;
                Progress = Math.Min(1, doneWeight / totalWeight);
                await Task.Yield();
            }

            if (!token.IsCancellationRequested)
            {
                AddLog(
                    L("检测完成", "Detection finished"),
                    L($"已检测: {TotalDetected}, 发现水印: {TotalFound}", $"Processed: {TotalDetected}, found: {TotalFound}"),
                    true,
                    false,
                    null,
                    LogIconTone.Info);
                if (TotalDetected > 0)
                {
                    _ = FlashDetectSuccessAsync();
                }
            }
        }
        catch (Exception ex)
        {
            AddLog(L("检测失败", "Detection failed"), ex.Message, false, false, null, LogIconTone.Error);
        }
        finally
        {
            CurrentProcessingFile = null;
            CurrentProcessingIndex = -1;
            IsProcessing = false;
            ScheduleProgressResetIfNeeded();
            try
            {
                await AppViewModel.Instance.RefreshStatsAsync();
            }
            catch
            {
                // Ignore stats refresh errors to avoid UI crash.
            }
        }
    }

    private DetectRecord DetectSingleFile(
        string filePath,
        byte[]? key,
        AwmChannelLayout layout,
        Action<AwmBridge.ProgressSnapshot>? onProgress = null)
    {
        var (mcDetected, detectError) = AwmBridge.DetectAudioMultichannelDetailed(filePath, layout, onProgress);
        if (detectError == AwmError.Ok && mcDetected is AwmBridge.MultichannelDetectAudioResult mcResult)
        {
            var (singleDetected, singleDetectError) = AwmBridge.DetectAudioDetailed(filePath);
            var detectScore = singleDetectError == AwmError.Ok && singleDetected.HasValue
                ? singleDetected.Value.DetectScore
                : null;
            var pattern = DetectPatternText(layout, mcResult.PairCount, singleDetected);

            var (unverifiedDecoded, unverifiedError) = AwmBridge.DecodeMessageUnverified(mcResult.RawMessage);
            if (key is not null)
            {
                var (decoded, decodeError) = AwmBridge.DecodeMessage(mcResult.RawMessage, key);
                if (decodeError == AwmError.Ok && decoded.HasValue)
                {
                    var decodedValue = decoded.Value;

                    string cloneCheck = "unavailable";
                    double? cloneScore = null;
                    float? cloneMatchSeconds = null;
                    string? cloneReason = null;

                    var identity = decodedValue.GetIdentity();
                    var keySlot = decodedValue.KeySlot;

                    var (clone, cloneError) = AwmBridge.CloneCheckForFile(filePath, identity, keySlot);
                    if (cloneError == AwmError.Ok && clone.HasValue)
                    {
                        cloneCheck = CloneKindToString(clone.Value.Kind);
                        cloneScore = clone.Value.Score;
                        cloneMatchSeconds = clone.Value.MatchSeconds;
                        cloneReason = clone.Value.Reason;
                    }
                    else
                    {
                        cloneCheck = "unavailable";
                        cloneReason = DescribeAwmError(cloneError);
                    }

                    return new DetectRecord
                    {
                        FilePath = filePath,
                        Status = "ok",
                        Verification = "verified",
                        Tag = decodedValue.GetTag(),
                        Identity = identity,
                        Version = decodedValue.Version,
                        TimestampMinutes = decodedValue.TimestampMinutes,
                        TimestampUtc = decodedValue.TimestampUtc,
                        KeySlot = decodedValue.KeySlot,
                        Pattern = pattern,
                        DetectScore = detectScore,
                        BitErrors = mcResult.BitErrors,
                        MatchFound = true,
                        CloneCheck = cloneCheck,
                        CloneScore = cloneScore,
                        CloneMatchSeconds = cloneMatchSeconds,
                        CloneReason = cloneReason,
                    };
                }
            }

            var invalidReason = key is null ? "key_not_configured" : "hmac_verification_failed";
            return new DetectRecord
            {
                FilePath = filePath,
                Status = "invalid_hmac",
                Verification = "unverified",
                Tag = unverifiedDecoded?.GetTag(),
                Identity = unverifiedDecoded?.GetIdentity(),
                Version = unverifiedDecoded?.Version,
                TimestampMinutes = unverifiedDecoded?.TimestampMinutes,
                TimestampUtc = unverifiedDecoded?.TimestampUtc,
                KeySlot = unverifiedDecoded?.KeySlot,
                Pattern = pattern,
                DetectScore = detectScore,
                BitErrors = mcResult.BitErrors,
                MatchFound = true,
                Error = unverifiedError == AwmError.Ok ? invalidReason : $"{invalidReason};{DescribeAwmError(unverifiedError)}",
            };
        }

        if (detectError == AwmError.NoWatermarkFound)
        {
            return new DetectRecord
            {
                FilePath = filePath,
                Status = "not_found",
            };
        }

        return new DetectRecord
        {
            FilePath = filePath,
            Status = "error",
            Error = DescribeAwmError(detectError),
        };
    }

    private void LogDetectionOutcome(string fileName, DetectRecord record)
    {
        switch (record.Status)
        {
            case "ok":
            {
                var timeText = LocalTimestampDisplay(record);
                AddLog(
                    $"{L("成功", "Success")}: {fileName}",
                    L($"标签: {record.Identity ?? "-"} | 时间: {timeText} | 克隆: {record.CloneCheck ?? "-"}", $"Identity: {record.Identity ?? "-"} | Time: {timeText} | Clone: {record.CloneCheck ?? "-"}"),
                    true,
                    false,
                    record.Id,
                    LogIconTone.Success,
                    LogKind.ResultOk);
                break;
            }
            case "not_found":
                AddLog($"{L("无标记", "Not found")}: {fileName}", L("未检测到水印", "No watermark detected"), false, false, record.Id, LogIconTone.Warning, LogKind.ResultNotFound);
                break;
            case "invalid_hmac":
                var warningText = L(
                    "UNVERIFIED · 不可用于归属/取证",
                    "UNVERIFIED · Do not use for attribution/forensics");
                AddLog(
                    $"{L("失败", "Failed")}: {fileName}",
                    L(
                        $"HMAC 校验失败: {record.Error ?? "unknown"} · {warningText}",
                        $"HMAC verification failed: {record.Error ?? "unknown"} · {warningText}"
                    ),
                    false,
                    false,
                    record.Id,
                    LogIconTone.Error,
                    LogKind.ResultInvalidHmac);
                break;
            default:
                AddLog($"{L("失败", "Failed")}: {fileName}", record.Error ?? L("未知错误", "Unknown error"), false, false, record.Id, LogIconTone.Error, LogKind.ResultError);
                break;
        }
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
                    AddLog(L("目录无可用音频", "No audio files in directory"), BuildDirectoryNoAudioDetail(), false, true, null, LogIconTone.Warning);
                }

                return supported;
            }
            catch (Exception ex)
            {
                AddLog(L("读取目录失败", "Failed to read directory"), ex.Message, false, false, null, LogIconTone.Error);
                return Array.Empty<string>();
            }
        }

        if (File.Exists(sourcePath) && IsSupportedAudioFile(sourcePath))
        {
            return new[] { sourcePath };
        }

        AddLog(L("不支持的输入源", "Unsupported input source"), BuildUnsupportedInputDetail(), false, true, null, LogIconTone.Warning);
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
            AddLog(L("已去重", "Deduplicated"), L($"跳过 {duplicateCount} 个重复文件", $"Skipped {duplicateCount} duplicate files"), true, true, null, LogIconTone.Info);
        }
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
            null,
            LogIconTone.Warning
        );
    }

    private void InsertDetectRecord(DetectRecord record)
    {
        DetectRecords.Insert(0, record);
        while (DetectRecords.Count > MaxLogCount)
        {
            DetectRecords.RemoveAt(DetectRecords.Count - 1);
        }

        HideDetectDetailWhenNoSelection = false;
        NotifyDisplayedRecordChanged();
    }

    private void AddLog(
        string title,
        string detail = "",
        bool isSuccess = true,
        bool isEphemeral = false,
        Guid? relatedRecordId = null,
        LogIconTone iconTone = LogIconTone.Info,
        LogKind kind = LogKind.Generic)
    {
        var entry = new LogEntry
        {
            Title = title,
            Detail = detail,
            IsSuccess = isSuccess,
            IsEphemeral = isEphemeral,
            RelatedRecordId = relatedRecordId,
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

    private async Task FlashInputSelectAsync()
    {
        IsInputSelectSuccess = true;
        await Task.Delay(TimeSpan.FromMilliseconds(900));
        IsInputSelectSuccess = false;
    }

    private async Task FlashDetectSuccessAsync()
    {
        IsDetectSuccess = true;
        await Task.Delay(TimeSpan.FromMilliseconds(900));
        IsDetectSuccess = false;
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
        OnPropertyChanged(nameof(CanDetectOrStop));
        OnPropertyChanged(nameof(QueueCount));
        OnPropertyChanged(nameof(HasQueueCount));
        OnPropertyChanged(nameof(QueueCountText));
        OnPropertyChanged(nameof(HasQueueFiles));
        OnPropertyChanged(nameof(ShowQueueEmptyHint));
    }

    private void OnLogsChanged(object? sender, NotifyCollectionChangedEventArgs e)
    {
        if (Logs.Count == 0)
        {
            if (!string.IsNullOrEmpty(LogSearchText))
            {
                _logSearchText = string.Empty;
                OnPropertyChanged(nameof(LogSearchText));
            }

            SelectedResultLogId = null;
            HideDetectDetailWhenNoSelection = false;
        }
        else if (SelectedResultLogId.HasValue && Logs.All(x => x.Id != SelectedResultLogId.Value))
        {
            SelectedResultLogId = null;
            HideDetectDetailWhenNoSelection = false;
        }

        NotifyFilteredLogsChanged();
        RefreshLogSelectionState();
    }

    private void OnDetectRecordsChanged(object? sender, NotifyCollectionChangedEventArgs e)
    {
        if (DetectRecords.Count == 0)
        {
            HideDetectDetailWhenNoSelection = false;
        }

        NotifyDisplayedRecordChanged();
    }

    private void NotifyFilteredLogsChanged()
    {
        OnPropertyChanged(nameof(FilteredLogs));
        OnPropertyChanged(nameof(HasLogs));
        OnPropertyChanged(nameof(LogCountText));
        OnPropertyChanged(nameof(ShowSearchBox));
        OnPropertyChanged(nameof(ShowNoLogsHint));
        OnPropertyChanged(nameof(HasFilteredLogs));
        OnPropertyChanged(nameof(ShowNoFilteredLogsHint));
    }

    private void RefreshLogSelectionState()
    {
        foreach (var log in Logs)
        {
            log.IsSelected = SelectedResultLogId.HasValue && log.Id == SelectedResultLogId.Value;
        }
    }

    private void NotifyDisplayedRecordChanged()
    {
        OnPropertyChanged(nameof(DisplayedRecord));
        OnPropertyChanged(nameof(DisplayFile));
        OnPropertyChanged(nameof(DisplayStatus));
        OnPropertyChanged(nameof(DisplayMatchFound));
        OnPropertyChanged(nameof(DisplayPattern));
        OnPropertyChanged(nameof(DisplayTag));
        OnPropertyChanged(nameof(DisplayIdentity));
        OnPropertyChanged(nameof(DisplayVersion));
        OnPropertyChanged(nameof(DisplayDetectTime));
        OnPropertyChanged(nameof(DisplayKeySlot));
        OnPropertyChanged(nameof(DisplayBitErrors));
        OnPropertyChanged(nameof(DisplayDetectScore));
        OnPropertyChanged(nameof(DisplayCloneCheck));
        OnPropertyChanged(nameof(DisplayFingerprintScore));
        OnPropertyChanged(nameof(DisplayError));
        OnPropertyChanged(nameof(StatusBrush));
        OnPropertyChanged(nameof(MatchFoundBrush));
        OnPropertyChanged(nameof(BitErrorsBrush));
        OnPropertyChanged(nameof(DetectScoreBrush));
        OnPropertyChanged(nameof(CloneCheckBrush));
        OnPropertyChanged(nameof(FingerprintScoreBrush));
        OnPropertyChanged(nameof(ErrorBrush));
    }

    private static string DetailValue(string? value)
    {
        return string.IsNullOrWhiteSpace(value) ? "-" : value;
    }

    private string StatusDisplayValue(string? status)
    {
        if (string.IsNullOrWhiteSpace(status))
        {
            return "-";
        }

        return status switch
        {
            "ok" => L("成功", "ok"),
            "not_found" => L("无标记", "not_found"),
            "invalid_hmac" => L("校验失败", "invalid_hmac"),
            "error" => L("错误", "error"),
            _ => status,
        };
    }

    private string CloneCheckDisplayValue(string? cloneCheck)
    {
        if (string.IsNullOrWhiteSpace(cloneCheck))
        {
            return "-";
        }

        return cloneCheck switch
        {
            "exact" => L("一致", "exact"),
            "likely" => L("疑似一致", "likely"),
            "suspect" => L("可疑", "suspect"),
            "unavailable" => L("不可用", "unavailable"),
            _ => cloneCheck,
        };
    }

    private static string CloneKindToString(AwmCloneCheckKind kind)
    {
        return kind switch
        {
            AwmCloneCheckKind.Exact => "exact",
            AwmCloneCheckKind.Likely => "likely",
            AwmCloneCheckKind.Suspect => "suspect",
            _ => "unavailable",
        };
    }

    private string DetectPatternText(
        AwmChannelLayout layout,
        uint pairCount,
        AwmBridge.DetectAudioResult? singleDetected)
    {
        if (singleDetected is AwmBridge.DetectAudioResult detailed &&
            !string.IsNullOrWhiteSpace(detailed.Pattern) &&
            layout == AwmChannelLayout.Stereo)
        {
            return detailed.Pattern;
        }

        return layout switch
        {
            AwmChannelLayout.Stereo => "stereo",
            AwmChannelLayout.Surround51 => L($"multichannel 5.1 ({pairCount} 对)", $"multichannel 5.1 ({pairCount} pairs)"),
            AwmChannelLayout.Surround512 => L($"multichannel 5.1.2 ({pairCount} 对)", $"multichannel 5.1.2 ({pairCount} pairs)"),
            AwmChannelLayout.Surround71 => L($"multichannel 7.1 ({pairCount} 对)", $"multichannel 7.1 ({pairCount} pairs)"),
            AwmChannelLayout.Surround714 => L($"multichannel 7.1.4 ({pairCount} 对)", $"multichannel 7.1.4 ({pairCount} pairs)"),
            AwmChannelLayout.Surround916 => L($"multichannel 9.1.6 ({pairCount} 对)", $"multichannel 9.1.6 ({pairCount} pairs)"),
            _ => L($"multichannel auto ({pairCount} 对)", $"multichannel auto ({pairCount} pairs)"),
        };
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

    private static ChannelLayoutOption CreateLayoutOption(AwmChannelLayout layout, string label)
    {
        return new ChannelLayoutOption(layout, label, AwmBridge.GetLayoutChannels(layout));
    }

    private static string FingerprintScoreDisplay(DetectRecord? record)
    {
        if (record?.CloneScore is not double score)
        {
            return "-";
        }

        if (record.CloneMatchSeconds is float seconds)
        {
            return $"{score:0.00} / {seconds:0}s";
        }

        return $"{score:0.00}";
    }

    private string ErrorDisplayValue(DetectRecord? record)
    {
        if (record?.Verification == "unverified")
        {
            var warning = L(
                "UNVERIFIED · 不可用于归属/取证",
                "UNVERIFIED · Do not use for attribution/forensics");
            if (!string.IsNullOrWhiteSpace(record.Error))
            {
                return $"{warning} · {record.Error}";
            }

            return warning;
        }

        if (!string.IsNullOrWhiteSpace(record?.Error))
        {
            return record.Error;
        }

        if (!string.IsNullOrWhiteSpace(record?.CloneReason))
        {
            return $"{L("克隆", "clone")}: {record.CloneReason}";
        }

        return "-";
    }

    private static string LocalTimestampDisplay(DetectRecord? record)
    {
        if (record is null)
        {
            return "-";
        }

        ulong utcSeconds;
        if (record.TimestampMinutes.HasValue)
        {
            utcSeconds = record.TimestampUtc ?? (ulong)record.TimestampMinutes.Value * 60;
        }
        else if (record.TimestampUtc.HasValue)
        {
            utcSeconds = record.TimestampUtc.Value;
        }
        else
        {
            return "-";
        }

        var dt = DateTimeOffset.FromUnixTimeSeconds((long)utcSeconds).ToLocalTime().DateTime;
        return dt.ToString("yyyy-MM-dd HH:mm");
    }

    private enum FieldSemantic
    {
        Status,
        MatchFound,
        BitErrors,
        DetectScore,
        CloneCheck,
        FingerprintScore,
        Error,
    }

    private Brush FieldBrush(FieldSemantic semantic, DetectRecord? record)
    {
        switch (semantic)
        {
            case FieldSemantic.Status:
                return record?.Status switch
                {
                    "ok" => SuccessBrush,
                    "not_found" => SecondaryBrush,
                    "invalid_hmac" => WarningBrush,
                    "error" => ErrorColorBrush,
                    _ => SecondaryBrush,
                };

            case FieldSemantic.MatchFound:
                return record?.MatchFound switch
                {
                    true => SuccessBrush,
                    false => SecondaryBrush,
                    _ => SecondaryBrush,
                };

            case FieldSemantic.BitErrors:
                if (record?.BitErrors is not uint bitErrors)
                {
                    return SecondaryBrush;
                }

                if (bitErrors == 0)
                {
                    return SuccessBrush;
                }

                if (bitErrors <= 3)
                {
                    return WarningBrush;
                }

                return ErrorColorBrush;

            case FieldSemantic.DetectScore:
                if (record?.DetectScore is not float detectScore)
                {
                    return SecondaryBrush;
                }

                if (detectScore >= 1.30f)
                {
                    return SuccessBrush;
                }

                if (detectScore >= 1.10f)
                {
                    return WarningBrush;
                }

                if (detectScore >= 1.00f)
                {
                    return YellowBrush;
                }

                return ErrorColorBrush;

            case FieldSemantic.CloneCheck:
                return record?.CloneCheck switch
                {
                    "exact" => SuccessBrush,
                    "likely" => InfoBrush,
                    "suspect" => WarningBrush,
                    "unavailable" => SecondaryBrush,
                    _ => SecondaryBrush,
                };

            case FieldSemantic.FingerprintScore:
                if (record?.CloneScore is not double fpScore)
                {
                    return SecondaryBrush;
                }

                if (fpScore <= 1)
                {
                    return SuccessBrush;
                }

                if (fpScore <= 3)
                {
                    return InfoBrush;
                }

                if (fpScore <= 7)
                {
                    return WarningBrush;
                }

                return ErrorColorBrush;

            case FieldSemantic.Error:
                return DisplayError == "-" ? SecondaryBrush : ErrorColorBrush;

            default:
                return PrimaryBrush;
        }
    }

    private static Brush ThemeBrush(string key, string fallbackKey)
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

    private void RebuildLayoutOptions()
    {
        var selectedLayout = SelectedChannelLayout?.Layout ?? AwmChannelLayout.Auto;
        var options = BuildChannelLayoutOptions();
        ChannelLayoutOptions.Clear();
        foreach (var option in options)
        {
            ChannelLayoutOptions.Add(option);
        }

        SelectedChannelLayout = ChannelLayoutOptions.FirstOrDefault(x => x.Layout == selectedLayout)
            ?? ChannelLayoutOptions.FirstOrDefault();
    }

    private void NotifyLocalizedTextChanged()
    {
        OnPropertyChanged(nameof(InputSectionTitle));
        OnPropertyChanged(nameof(MissingKeyMessage));
        OnPropertyChanged(nameof(GoToKeyPageText));
        OnPropertyChanged(nameof(SelectActionText));
        OnPropertyChanged(nameof(ClearActionText));
        OnPropertyChanged(nameof(LayoutLabel));
        OnPropertyChanged(nameof(DropZoneTitle));
        OnPropertyChanged(nameof(DropZoneSubtitle));
        OnPropertyChanged(nameof(DetectInfoTitle));
        OnPropertyChanged(nameof(FileFieldLabel));
        OnPropertyChanged(nameof(StatusFieldLabel));
        OnPropertyChanged(nameof(MatchFieldLabel));
        OnPropertyChanged(nameof(PatternFieldLabel));
        OnPropertyChanged(nameof(TagFieldLabel));
        OnPropertyChanged(nameof(IdentityFieldLabel));
        OnPropertyChanged(nameof(VersionFieldLabel));
        OnPropertyChanged(nameof(TimeFieldLabel));
        OnPropertyChanged(nameof(KeySlotFieldLabel));
        OnPropertyChanged(nameof(BitErrorsFieldLabel));
        OnPropertyChanged(nameof(DetectScoreFieldLabel));
        OnPropertyChanged(nameof(CloneCheckFieldLabel));
        OnPropertyChanged(nameof(FingerprintScoreFieldLabel));
        OnPropertyChanged(nameof(ErrorFieldLabel));
        OnPropertyChanged(nameof(QueueTitle));
        OnPropertyChanged(nameof(QueueEmptyText));
        OnPropertyChanged(nameof(LogsTitle));
        OnPropertyChanged(nameof(LogSearchPlaceholder));
        OnPropertyChanged(nameof(NoFilteredLogsText));
        OnPropertyChanged(nameof(SelectInputSourceAccessibility));
        OnPropertyChanged(nameof(ClearInputSourceAccessibility));
        OnPropertyChanged(nameof(DetectActionAccessibility));
        OnPropertyChanged(nameof(ClearQueueAccessibility));
        OnPropertyChanged(nameof(ClearLogsAccessibility));
    }

    private static Dictionary<string, double> BuildProgressWeights(IEnumerable<string> files)
    {
        var result = new Dictionary<string, double>(StringComparer.OrdinalIgnoreCase);
        foreach (var file in files)
        {
            var key = NormalizedPathKey(file);
            double weight;
            try
            {
                var info = new FileInfo(file);
                weight = info.Exists ? Math.Max(info.Length, 1) : 1;
            }
            catch
            {
                weight = 1;
            }

            result[key] = weight;
        }

        return result;
    }

    private static double MapSnapshotProgress(
        AwmBridge.ProgressSnapshot snapshot,
        ProgressProfile profile,
        ref AwmProgressPhase lastPhase,
        double previous)
    {
        if (snapshot.State == AwmProgressState.Completed)
        {
            return 1;
        }

        var (rangeStart, rangeEnd) = PhaseInterval(snapshot.Phase, profile);
        if (snapshot.Determinate && snapshot.TotalUnits > 0)
        {
            var ratio = Math.Clamp(snapshot.CompletedUnits / (double)snapshot.TotalUnits, 0, 1);
            var mapped = rangeStart + ((rangeEnd - rangeStart) * ratio);
            return Math.Clamp(Math.Max(previous, mapped), 0, 1);
        }

        var cap = Math.Max(rangeStart, rangeEnd - Math.Max((rangeEnd - rangeStart) * 0.08, 0.01));
        var step = snapshot.Phase == lastPhase ? 0.0035 : 0.0015;
        lastPhase = snapshot.Phase;
        var baseline = Math.Max(previous, rangeStart);
        return Math.Clamp(Math.Min(cap, baseline + step), 0, 1);
    }

    private static (double start, double end) PhaseInterval(AwmProgressPhase phase, ProgressProfile profile)
    {
        return profile switch
        {
            ProgressProfile.Embed => phase switch
            {
                AwmProgressPhase.PrepareInput or AwmProgressPhase.Precheck => (0.00, 0.15),
                AwmProgressPhase.Core or AwmProgressPhase.RouteStep or AwmProgressPhase.Merge => (0.15, 0.85),
                AwmProgressPhase.Evidence or AwmProgressPhase.CloneCheck => (0.85, 0.95),
                AwmProgressPhase.Finalize => (0.95, 1.00),
                _ => (0.0, 0.0),
            },
            _ => phase switch
            {
                AwmProgressPhase.PrepareInput or AwmProgressPhase.Precheck => (0.00, 0.10),
                AwmProgressPhase.Core or AwmProgressPhase.RouteStep or AwmProgressPhase.Merge => (0.10, 0.80),
                AwmProgressPhase.Evidence or AwmProgressPhase.CloneCheck => (0.80, 0.95),
                AwmProgressPhase.Finalize => (0.95, 1.00),
                _ => (0.0, 0.0),
            },
        };
    }

    private enum ProgressProfile
    {
        Embed,
        Detect,
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
                "ADM/BWF 暂不支持检测或元数据结构不受支持",
                "ADM/BWF detect is not supported yet or metadata layout is unsupported"
            ),
            AwmError.AdmPreserveFailed => L(
                "ADM/BWF 元数据保真失败",
                "Failed to preserve ADM/BWF metadata"
            ),
            AwmError.AdmPcmFormatUnsupported => L(
                "ADM/BWF PCM 格式不支持：仅支持 16/24/32-bit PCM",
                "Unsupported ADM/BWF PCM format: only 16/24/32-bit PCM"
            ),
            _ => error.ToString(),
        };
    }
}
