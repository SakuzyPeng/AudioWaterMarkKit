using AWMKit.Models;
using AWMKit.Native;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using Microsoft.UI.Xaml.Media;
using System;
using System.Collections.Generic;
using System.Collections.ObjectModel;
using System.Collections.Specialized;
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

    private static readonly SolidColorBrush SuccessBrush = new(Windows.UI.Color.FromArgb(255, 76, 175, 80));
    private static readonly SolidColorBrush WarningBrush = new(Windows.UI.Color.FromArgb(255, 255, 152, 0));
    private static readonly SolidColorBrush ErrorColorBrush = new(Windows.UI.Color.FromArgb(255, 244, 67, 54));
    private static readonly SolidColorBrush InfoBrush = new(Windows.UI.Color.FromArgb(255, 33, 150, 243));
    private static readonly SolidColorBrush SecondaryBrush = new(Windows.UI.Color.FromArgb(255, 158, 158, 158));
    private static readonly SolidColorBrush PrimaryBrush = new(Windows.UI.Color.FromArgb(255, 240, 240, 240));
    private static readonly SolidColorBrush YellowBrush = new(Windows.UI.Color.FromArgb(255, 255, 213, 79));

    private readonly HashSet<string> _supportedAudioExtensions = new(StringComparer.OrdinalIgnoreCase)
    {
        ".wav",
        ".flac",
    };

    private CancellationTokenSource? _detectCts;
    private CancellationTokenSource? _progressResetCts;

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

    public string InputSourceText => string.IsNullOrWhiteSpace(InputSource) ? "尚未选择输入源" : InputSource;

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

    public string DetectCountText => $"{TotalFound}（成功）/{TotalDetected}（总）";

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

    public string LogCountText => $"共 {Logs.Count} 条";

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

    public string QueueCountText => $"共 {QueueCount} 个";

    public bool CanDetectOrStop => IsProcessing || SelectedFiles.Count > 0;

    public string DetectButtonText => IsProcessing ? "停止" : "检测";
    public bool ShowDetectStopIcon => IsProcessing;
    public bool ShowDetectDefaultPlayIcon => !IsProcessing && !IsDetectSuccess;
    public bool ShowDetectSuccessPlayIcon => !IsProcessing && IsDetectSuccess;

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

    public string DisplayStatus => DetailValue(DisplayedRecord?.Status);

    public string DisplayMatchFound => DisplayedRecord?.MatchFound switch
    {
        true => "true",
        false => "false",
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

    public string DisplayCloneCheck => DetailValue(DisplayedRecord?.CloneCheck);

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
        var files = filePaths
            .Where(IsSupportedAudioFile)
            .ToList();

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
    private async Task ClearQueueAsync()
    {
        if (!SelectedFiles.Any())
        {
            AddLog("队列为空", "没有可移除的文件", true, true, null, LogIconTone.Info);
            return;
        }

        var count = SelectedFiles.Count;
        SelectedFiles.Clear();
        AddLog("已清空队列", $"移除了 {count} 个文件", true, false, null, LogIconTone.Success);
        await FlashClearQueueAsync();
    }

    [RelayCommand]
    private async Task ClearLogsAsync()
    {
        if (!Logs.Any())
        {
            AddLog("日志为空", "没有可清空的日志", true, true, null, LogIconTone.Info);
            return;
        }

        var count = Logs.Count;
        Logs.Clear();
        DetectRecords.Clear();
        TotalDetected = 0;
        TotalFound = 0;

        AddLog("已清空日志", $"移除了 {count} 条日志记录", true, true, null, LogIconTone.Success);
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
                AddLog("检测已停止", "用户手动停止", false, true, null, LogIconTone.Warning);
                return;
            }

            await DetectAsync();
        }
        catch (Exception ex)
        {
            AddLog("检测异常", ex.Message, false, false, null, LogIconTone.Error);
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
            AddLog("队列为空", "请先添加音频文件", false, true, null, LogIconTone.Warning);
            return;
        }

        try
        {
            var (key, keyError) = AwmKeyBridge.LoadKey();
            if (keyError != AwmError.Ok || key is null)
            {
                AddLog("检测失败", "密钥未配置", false, false, null, LogIconTone.Error);
                return;
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
            var layoutText = SelectedChannelLayout?.DisplayText ?? "自动";

            AddLog("开始检测", $"准备检测 {SelectedFiles.Count} 个文件（{layoutText}）", true, false, null, LogIconTone.Info);

            var initialTotal = SelectedFiles.Count;
            for (var processed = 0; processed < initialTotal; processed++)
            {
                if (token.IsCancellationRequested || SelectedFiles.Count == 0)
                {
                    break;
                }

                var filePath = SelectedFiles[0];
                var fileName = Path.GetFileName(filePath);
                CurrentProcessingFile = fileName;
                CurrentProcessingIndex = 0;

                DetectRecord record;
                try
                {
                    record = await Task.Run(() => DetectSingleFile(filePath, key, channelLayout), token);
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

                Progress = (processed + 1) / (double)initialTotal;
            }

            if (!token.IsCancellationRequested)
            {
                AddLog("检测完成", $"已检测: {TotalDetected}, 发现水印: {TotalFound}", true, false, null, LogIconTone.Info);
                if (TotalDetected > 0)
                {
                    _ = FlashDetectSuccessAsync();
                }
            }
        }
        catch (Exception ex)
        {
            AddLog("检测失败", ex.Message, false, false, null, LogIconTone.Error);
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

    private DetectRecord DetectSingleFile(string filePath, byte[] key, AwmChannelLayout layout)
    {
        var (mcDetected, detectError) = AwmBridge.DetectAudioMultichannelDetailed(filePath, layout);
        if (detectError == AwmError.Ok && mcDetected is AwmBridge.MultichannelDetectAudioResult mcResult)
        {
            var (singleDetected, singleDetectError) = AwmBridge.DetectAudioDetailed(filePath);
            var detectScore = singleDetectError == AwmError.Ok && singleDetected.HasValue
                ? singleDetected.Value.DetectScore
                : null;
            var pattern = DetectPatternText(layout, mcResult.PairCount, singleDetected);

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
                    cloneReason = cloneError.ToString();
                }

                return new DetectRecord
                {
                    FilePath = filePath,
                    Status = "ok",
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

            return new DetectRecord
            {
                FilePath = filePath,
                Status = "invalid_hmac",
                Pattern = pattern,
                DetectScore = detectScore,
                BitErrors = mcResult.BitErrors,
                MatchFound = true,
                Error = decodeError.ToString(),
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
            Error = detectError.ToString(),
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
                    $"成功: {fileName}",
                    $"标签: {record.Identity ?? "-"} | 时间: {timeText} | 克隆: {record.CloneCheck ?? "-"}",
                    true,
                    false,
                    record.Id,
                    LogIconTone.Success);
                break;
            }
            case "not_found":
                AddLog($"无标记: {fileName}", "未检测到水印", false, false, record.Id, LogIconTone.Warning);
                break;
            case "invalid_hmac":
                AddLog(
                    $"失败: {fileName}",
                    $"HMAC 校验失败: {record.Error ?? "unknown"}",
                    false,
                    false,
                    record.Id,
                    LogIconTone.Error);
                break;
            default:
                AddLog($"失败: {fileName}", record.Error ?? "未知错误", false, false, record.Id, LogIconTone.Error);
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
                    .Where(IsSupportedAudioFile)
                    .ToList();

                if (files.Count == 0)
                {
                    AddLog("目录无可用音频", "当前目录未找到 WAV / FLAC 文件", false, true, null, LogIconTone.Warning);
                }

                return files;
            }
            catch (Exception ex)
            {
                AddLog("读取目录失败", ex.Message, false, false, null, LogIconTone.Error);
                return Array.Empty<string>();
            }
        }

        if (File.Exists(sourcePath) && IsSupportedAudioFile(sourcePath))
        {
            return new[] { sourcePath };
        }

        AddLog("不支持的输入源", "请选择 WAV / FLAC 文件或包含这些文件的目录", false, true, null, LogIconTone.Warning);
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
            AddLog("已去重", $"跳过 {duplicateCount} 个重复文件", true, true, null, LogIconTone.Info);
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
        return _supportedAudioExtensions.Contains(ext);
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
        LogIconTone iconTone = LogIconTone.Info)
    {
        var entry = new LogEntry
        {
            Title = title,
            Detail = detail,
            IsSuccess = isSuccess,
            IsEphemeral = isEphemeral,
            RelatedRecordId = relatedRecordId,
            IconTone = iconTone,
        };

        Logs.Insert(0, entry);
        while (Logs.Count > MaxLogCount)
        {
            Logs.RemoveAt(Logs.Count - 1);
        }

        if (entry.IsEphemeral && entry.Title == "已清空日志")
        {
            _ = DismissClearLogAsync(entry.Id);
        }
    }

    private async Task DismissClearLogAsync(Guid logId)
    {
        await Task.Delay(TimeSpan.FromSeconds(3));

        var target = Logs.FirstOrDefault(x => x.Id == logId && x.Title == "已清空日志");
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

    private static string DetectPatternText(
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
            AwmChannelLayout.Surround51 => $"multichannel 5.1 ({pairCount} 对)",
            AwmChannelLayout.Surround512 => $"multichannel 5.1.2 ({pairCount} 对)",
            AwmChannelLayout.Surround71 => $"multichannel 7.1 ({pairCount} 对)",
            AwmChannelLayout.Surround714 => $"multichannel 7.1.4 ({pairCount} 对)",
            AwmChannelLayout.Surround916 => $"multichannel 9.1.6 ({pairCount} 对)",
            _ => $"multichannel auto ({pairCount} 对)",
        };
    }

    private static ObservableCollection<ChannelLayoutOption> BuildChannelLayoutOptions()
    {
        return new ObservableCollection<ChannelLayoutOption>
        {
            CreateLayoutOption(AwmChannelLayout.Auto, "自动"),
            CreateLayoutOption(AwmChannelLayout.Stereo, "立体声"),
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

    private static string ErrorDisplayValue(DetectRecord? record)
    {
        if (!string.IsNullOrWhiteSpace(record?.Error))
        {
            return record.Error;
        }

        if (!string.IsNullOrWhiteSpace(record?.CloneReason))
        {
            return $"clone: {record.CloneReason}";
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
}
