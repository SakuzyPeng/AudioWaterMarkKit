using AWMKit.Models;
using AWMKit.Native;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using System.Collections.ObjectModel;
using System.IO;
using System.Linq;
using System.Threading.Tasks;

namespace AWMKit.ViewModels;

/// <summary>
/// View model for the Detect page.
/// </summary>
public sealed partial class DetectViewModel : ObservableObject
{
    [ObservableProperty]
    private bool _isProcessing;

    [ObservableProperty]
    private int _processedCount;

    [ObservableProperty]
    private int _totalCount;

    [ObservableProperty]
    private string? _currentFile;

    [ObservableProperty]
    private bool _autoSaveEvidence = false;

    public ObservableCollection<string> InputFiles { get; } = new();
    public ObservableCollection<DetectResult> Results { get; } = new();

    [RelayCommand]
    private void AddFiles(string[] files)
    {
        foreach (var file in files)
        {
            if (!InputFiles.Contains(file))
            {
                InputFiles.Add(file);
            }
        }
    }

    [RelayCommand]
    private void RemoveFile(string file)
    {
        InputFiles.Remove(file);
    }

    [RelayCommand]
    private void ClearAll()
    {
        InputFiles.Clear();
        Results.Clear();
    }

    [RelayCommand]
    private async Task DetectAsync()
    {
        if (!InputFiles.Any())
        {
            return;
        }

        IsProcessing = true;
        ProcessedCount = 0;
        TotalCount = InputFiles.Count;
        Results.Clear();

        await Task.Run(async () =>
        {
            var (loadedKey, keyError) = AwmKeyBridge.LoadKey();
            var hasKey = loadedKey is not null && keyError == AwmError.Ok;

            foreach (var inputFile in InputFiles.ToList())
            {
                CurrentFile = Path.GetFileName(inputFile);
                var (detected, detectError) = AwmBridge.DetectAudioDetailed(inputFile);

                DetectResult result;
                if (detectError == AwmError.Ok && detected is not null)
                {
                    result = await BuildSuccessResultAsync(inputFile, detected.Value, hasKey ? loadedKey : null);
                }
                else
                {
                    result = new DetectResult
                    {
                        FilePath = inputFile,
                        Success = false,
                        Tag = null,
                        Identity = null,
                        KeySlot = null,
                        TimestampMinutes = null,
                        Pattern = null,
                        BitErrors = null,
                        DetectScore = null,
                        CloneCheck = null,
                        CloneScore = null,
                        CloneMatchSeconds = null,
                        CloneEvidenceId = null,
                        CloneReason = null,
                        Message = null,
                        Error = detectError,
                        ErrorMessage = detectError.ToString()
                    };
                }

                App.Current.MainWindow?.DispatcherQueue.TryEnqueue(() => Results.Add(result));
                ProcessedCount++;
            }

            CurrentFile = null;
            IsProcessing = false;
            await AppViewModel.Instance.RefreshStatsAsync();
        });
    }

    private static Task<DetectResult> BuildSuccessResultAsync(
        string inputFile,
        AwmBridge.DetectAudioResult detectResult,
        byte[]? key)
    {
        string? tag = null;
        string? identity = null;
        byte? keySlot = null;
        uint? timestampMinutes = null;
        AwmError? decodeError = null;
        string? decodeErrorMessage = null;

        if (key is not null)
        {
            var (decoded, decodeErr) = AwmBridge.DecodeMessage(detectResult.RawMessage, key);
            if (decodeErr == AwmError.Ok && decoded.HasValue)
            {
                tag = decoded.Value.GetTag();
                identity = decoded.Value.GetIdentity();
                keySlot = decoded.Value.KeySlot;
                timestampMinutes = decoded.Value.TimestampMinutes;
            }
            else
            {
                decodeError = decodeErr;
                decodeErrorMessage = decodeErr.ToString();
            }
        }

        AwmCloneCheckKind? cloneKind = null;
        double? cloneScore = null;
        float? cloneSeconds = null;
        long? cloneEvidenceId = null;
        string? cloneReason = null;

        if (!string.IsNullOrEmpty(identity) && keySlot.HasValue)
        {
            var (clone, cloneErr) = AwmBridge.CloneCheckForFile(inputFile, identity, keySlot.Value);
            if (cloneErr == AwmError.Ok && clone.HasValue)
            {
                cloneKind = clone.Value.Kind;
                cloneScore = clone.Value.Score;
                cloneSeconds = clone.Value.MatchSeconds;
                cloneEvidenceId = clone.Value.EvidenceId;
                cloneReason = clone.Value.Reason;
            }
            else
            {
                cloneKind = AwmCloneCheckKind.Unavailable;
                cloneReason = cloneErr.ToString();
            }
        }

        return Task.FromResult(new DetectResult
        {
            FilePath = inputFile,
            Success = true,
            Tag = tag,
            Identity = identity,
            KeySlot = keySlot,
            TimestampMinutes = timestampMinutes,
            Pattern = detectResult.Pattern,
            BitErrors = detectResult.BitErrors,
            DetectScore = detectResult.DetectScore,
            CloneCheck = cloneKind,
            CloneScore = cloneScore,
            CloneMatchSeconds = cloneSeconds,
            CloneEvidenceId = cloneEvidenceId,
            CloneReason = cloneReason,
            Message = detectResult.RawMessage,
            Error = decodeError,
            ErrorMessage = decodeErrorMessage
        });
    }

    public bool CanDetect => InputFiles.Any() && !IsProcessing;
}
