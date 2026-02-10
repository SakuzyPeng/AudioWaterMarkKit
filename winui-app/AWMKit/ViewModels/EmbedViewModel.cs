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

    private static readonly SolidColorBrush SuccessBrush = new(Windows.UI.Color.FromArgb(255, 76, 175, 80));
    private static readonly SolidColorBrush FallbackPrimaryBrush = new(Windows.UI.Color.FromArgb(255, 48, 48, 48));
    private static readonly SolidColorBrush FallbackSecondaryBrush = new(Windows.UI.Color.FromArgb(255, 130, 130, 130));

    private readonly HashSet<string> _supportedAudioExtensions = new(StringComparer.OrdinalIgnoreCase)
    {
        ".wav",
        ".flac",
    };

    private CancellationTokenSource? _embedCts;
    private CancellationTokenSource? _progressResetCts;
    private bool _isUpdatingFromSelection;

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

    public string InputSourceText => string.IsNullOrWhiteSpace(InputSource) ? "尚未选择输入源" : InputSource;

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

    public string OutputDirectoryText => string.IsNullOrWhiteSpace(OutputDirectory) ? "默认写回各文件所在目录" : OutputDirectory;

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
            ? SuccessBrush
            : (PreviewTagText == "-" ? ThemeSecondaryBrush() : ThemePrimaryBrush());

    public ObservableCollection<string> SelectedFiles { get; } = new();
    public ObservableCollection<LogEntry> Logs { get; } = new();
    public ObservableCollection<EmbedMappingOption> AllMappings { get; } = new();
    public ObservableCollection<EmbedMappingOption> MappingSuggestions { get; } = new();

    public int QueueCount => SelectedFiles.Count;
    public bool HasQueueCount => QueueCount > 0;
    public string QueueCountText => $"共 {QueueCount} 个";
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
    public string LogCountText => $"共 {Logs.Count} 条";
    public bool ShowNoLogsHint => !HasLogs;

    public bool HasMappings => AllMappings.Count > 0;
    public bool CanEmbedOrStop => IsKeyAvailable && (IsProcessing || SelectedFiles.Count > 0);
    public string EmbedButtonText => IsProcessing ? "停止" : "嵌入";
    public bool ShowEmbedStopIcon => IsProcessing;
    public bool ShowEmbedDefaultPlayIcon => !IsProcessing && !IsEmbedSuccess;
    public bool ShowEmbedSuccessPlayIcon => !IsProcessing && IsEmbedSuccess;

    public string PreviewTagText => ResolveTagValue() ?? "-";
    public string PreviewTagDisplay => $"Tag: {PreviewTagText}";
    public string? MatchedMappingHintText => MatchedMappingForInput is null ? null : "已存在映射，自动复用";
    public string ReuseHintText => MatchedMappingForInput is null ? string.Empty : "复用 ";
    public string MappingPlaceholderText => HasMappings ? "选择已存储映射" : "暂无已存储映射";

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
        _ = RefreshTagMappingsAsync();
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

        SelectedFiles.Remove(filePath);
    }

    [RelayCommand]
    private async Task ClearQueueAsync()
    {
        if (!SelectedFiles.Any())
        {
            AddLog("队列为空", "没有可移除的文件", true, true, LogIconTone.Info);
            return;
        }

        var count = SelectedFiles.Count;
        SelectedFiles.Clear();
        AddLog("已清空队列", $"移除了 {count} 个文件", true, false, LogIconTone.Success);
        await FlashClearQueueAsync();
    }

    [RelayCommand]
    private async Task ClearLogsAsync()
    {
        if (!Logs.Any())
        {
            AddLog("日志为空", "没有可清空的日志", true, true, LogIconTone.Info);
            return;
        }

        var count = Logs.Count;
        Logs.Clear();
        AddLog("已清空日志", $"移除了 {count} 条日志记录", true, true, LogIconTone.Success);
        await FlashClearLogsAsync();
    }

    [RelayCommand]
    private async Task EmbedOrStopAsync()
    {
        if (IsProcessing)
        {
            _embedCts?.Cancel();
            IsCancelling = true;
            AddLog("嵌入已停止", "用户手动停止", false, true, LogIconTone.Warning);
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
            AddLog("队列为空", "请先添加音频文件", false, true, LogIconTone.Warning);
            return;
        }

        var username = NormalizedUsernameInput();
        if (string.IsNullOrEmpty(username))
        {
            AddLog("用户名未填写", "请输入用户名以自动生成 Tag", false, true, LogIconTone.Warning);
            return;
        }

        var resolvedTag = ResolveTagValue();
        if (string.IsNullOrWhiteSpace(resolvedTag))
        {
            AddLog("Tag 生成失败", "请检查用户名输入", false, false, LogIconTone.Error);
            return;
        }

        var (key, keyError) = AwmKeyBridge.LoadKey();
        if (key is null || keyError != AwmError.Ok)
        {
            AddLog("嵌入失败", $"密钥不可用: {keyError}", false, false, LogIconTone.Error);
            return;
        }

        var (message, encodeError) = AwmBridge.EncodeMessage(resolvedTag, key);
        if (message is null || encodeError != AwmError.Ok)
        {
            AddLog("嵌入失败", $"消息编码失败: {encodeError}", false, false, LogIconTone.Error);
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
        var layoutText = SelectedChannelLayout?.DisplayText ?? "自动";
        AddLog("开始处理", $"准备处理 {SelectedFiles.Count} 个文件（{layoutText}）", true, false, LogIconTone.Info);

        var initialTotal = SelectedFiles.Count;
        var successCount = 0;
        var failureCount = 0;

        for (var processed = 0; processed < initialTotal; processed++)
        {
            if (token.IsCancellationRequested || SelectedFiles.Count == 0)
            {
                break;
            }

            var inputPath = SelectedFiles[0];
            CurrentProcessingFile = Path.GetFileName(inputPath);
            CurrentProcessingIndex = 0;

            string outputPath;
            try
            {
                outputPath = BuildOutputPath(inputPath);
            }
            catch (Exception ex)
            {
                AddLog($"失败: {Path.GetFileName(inputPath)}", ex.Message, false, false, LogIconTone.Error);
                failureCount += 1;
                SelectedFiles.RemoveAt(0);
                Progress = (processed + 1) / (double)initialTotal;
                continue;
            }

            (AwmError embedError, AwmError evidenceError) stepResult;
            try
            {
                stepResult = await Task.Run(() =>
                {
                    var embed = AwmBridge.EmbedAudioMultichannel(
                        inputPath,
                        outputPath,
                        message,
                        layout,
                        Strength);

                    var evidence = AwmError.Ok;
                    if (embed == AwmError.Ok)
                    {
                        evidence = AwmBridge.RecordEvidenceFile(outputPath, message, key);
                    }

                    return (embed, evidence);
                }, token);
            }
            catch (OperationCanceledException)
            {
                break;
            }

            if (stepResult.embedError == AwmError.Ok)
            {
                if (stepResult.evidenceError != AwmError.Ok)
                {
                    AddLog(
                        "证据记录失败",
                        $"{Path.GetFileName(outputPath)}: {stepResult.evidenceError}",
                        false,
                        true,
                        LogIconTone.Warning);
                }

                successCount += 1;
                AddLog($"成功: {Path.GetFileName(inputPath)}", $"→ {Path.GetFileName(outputPath)}", true, false, LogIconTone.Success);
            }
            else
            {
                failureCount += 1;
                AddLog($"失败: {Path.GetFileName(inputPath)}", stepResult.embedError.ToString(), false, false, LogIconTone.Error);
            }

            if (SelectedFiles.Count > 0)
            {
                SelectedFiles.RemoveAt(0);
            }

            Progress = (processed + 1) / (double)initialTotal;
            await Task.Yield();
        }

        if (token.IsCancellationRequested)
        {
            AddLog("已取消", $"已完成 {successCount + failureCount} / {initialTotal} 个文件", false, false, LogIconTone.Warning);
        }
        else
        {
            AddLog("处理完成", $"成功: {successCount}, 失败: {failureCount}", true, false, LogIconTone.Info);
        }

        if (successCount > 0)
        {
            _ = FlashEmbedSuccessAsync();
            var inserted = await AppViewModel.Instance.TagStore.SaveIfAbsentAsync(username, resolvedTag);
            if (inserted)
            {
                await RefreshTagMappingsAsync();
                AddLog("已保存映射", $"{username} -> {resolvedTag}", true, false, LogIconTone.Success);
            }
        }

        CurrentProcessingFile = null;
        CurrentProcessingIndex = -1;
        IsProcessing = false;
        IsCancelling = false;
        ScheduleProgressResetIfNeeded();

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
            : Path.GetDirectoryName(inputPath) ?? throw new InvalidOperationException("无法确定输出目录");

        var suffix = string.IsNullOrWhiteSpace(CustomSuffix) ? "_wm" : CustomSuffix.Trim();
        var baseName = Path.GetFileNameWithoutExtension(inputPath);
        var ext = Path.GetExtension(inputPath);
        return Path.Combine(outputDirectory, $"{baseName}{suffix}{ext}");
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
                    AddLog("目录无可用音频", "当前目录未找到 WAV / FLAC 文件", false, true, LogIconTone.Warning);
                }

                return files;
            }
            catch (Exception ex)
            {
                AddLog("读取目录失败", ex.Message, false, false, LogIconTone.Error);
                return Array.Empty<string>();
            }
        }

        if (File.Exists(sourcePath) && IsSupportedAudioFile(sourcePath))
        {
            return new[] { sourcePath };
        }

        AddLog("不支持的输入源", "请选择 WAV / FLAC 文件或包含这些文件的目录", false, true, LogIconTone.Warning);
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
            AddLog("已去重", $"跳过 {duplicateCount} 个重复文件", true, true, LogIconTone.Info);
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
        LogIconTone iconTone = LogIconTone.Info)
    {
        var entry = new LogEntry
        {
            Title = title,
            Detail = detail,
            IsSuccess = isSuccess,
            IsEphemeral = isEphemeral,
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
        if (Application.Current.Resources.TryGetValue("TextFillColorPrimaryBrush", out var brush) && brush is Brush typed)
        {
            return typed;
        }
        return FallbackPrimaryBrush;
    }

    private static Brush ThemeSecondaryBrush()
    {
        if (Application.Current.Resources.TryGetValue("TextFillColorSecondaryBrush", out var brush) && brush is Brush typed)
        {
            return typed;
        }
        return FallbackSecondaryBrush;
    }
}
