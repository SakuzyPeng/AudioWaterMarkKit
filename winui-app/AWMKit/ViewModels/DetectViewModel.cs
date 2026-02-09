using AWMKit.Models;
using AWMKit.Native;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using System;
using System.Collections.ObjectModel;
using System.IO;
using System.Linq;
using System.Security.Cryptography;
using System.Text;
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
    private bool _autoSaveEvidence = true;

    public ObservableCollection<string> InputFiles { get; } = new();
    public ObservableCollection<DetectResult> Results { get; } = new();

    /// <summary>
    /// Adds files to the input list.
    /// </summary>
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

    /// <summary>
    /// Removes a file from the input list.
    /// </summary>
    [RelayCommand]
    private void RemoveFile(string file)
    {
        InputFiles.Remove(file);
    }

    /// <summary>
    /// Clears all input files and results.
    /// </summary>
    [RelayCommand]
    private void ClearAll()
    {
        InputFiles.Clear();
        Results.Clear();
    }

    /// <summary>
    /// Starts the detection process.
    /// </summary>
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
            foreach (var inputFile in InputFiles.ToList())
            {
                CurrentFile = Path.GetFileName(inputFile);

                // Calculate file hash
                var fileHash = ComputeFileHash(inputFile);

                // Detect watermark
                var (message, pattern, error) = AwmBridge.DetectAudio(inputFile);

                DetectResult result;

                if (error == AwmError.Ok && message is not null)
                {
                    // Try to decode tag from message
                    var tag = TryDecodeTag(message);

                    // Lookup user identity from tag mapping
                    TagMapping? mapping = null;
                    if (!string.IsNullOrEmpty(tag))
                    {
                        mapping = await AppViewModel.Instance.TagStore.GetByTagAsync(tag);
                    }

                    result = new DetectResult
                    {
                        FilePath = inputFile,
                        FileHash = fileHash,
                        Success = true,
                        Tag = tag,
                        Identity = mapping?.Identity,
                        DisplayName = mapping?.DisplayName,
                        Pattern = pattern,
                        Message = message,
                        Error = null,
                        ErrorMessage = null
                    };

                    // Auto-save evidence
                    if (AutoSaveEvidence && !string.IsNullOrEmpty(tag))
                    {
                        var messageHex = BitConverter.ToString(message).Replace("-", "");
                        await AppViewModel.Instance.EvidenceStore.SaveAsync(
                            inputFile, fileHash, messageHex, pattern, tag);
                    }
                }
                else
                {
                    result = new DetectResult
                    {
                        FilePath = inputFile,
                        FileHash = fileHash,
                        Success = false,
                        Tag = null,
                        Identity = null,
                        DisplayName = null,
                        Pattern = null,
                        Message = null,
                        Error = error,
                        ErrorMessage = error.ToString()
                    };
                }

                // Must add to ObservableCollection on UI thread
                App.Current.MainWindow?.DispatcherQueue.TryEnqueue(() =>
                {
                    Results.Add(result);
                });

                ProcessedCount++;
            }

            CurrentFile = null;
            IsProcessing = false;

            // Refresh app stats
            await AppViewModel.Instance.RefreshStatsAsync();
        });
    }

    private static string ComputeFileHash(string filePath)
    {
        using var sha256 = SHA256.Create();
        using var stream = File.OpenRead(filePath);
        var hashBytes = sha256.ComputeHash(stream);
        return BitConverter.ToString(hashBytes).Replace("-", "").ToLowerInvariant();
    }

    private static string? TryDecodeTag(byte[] message)
    {
        // Try to decode with global key
        var (key, keyError) = AwmKeyBridge.LoadKey();
        if (key is null || keyError != AwmError.Ok)
        {
            return null;
        }

        var (result, decodeError) = AwmBridge.DecodeMessage(message, key);
        if (decodeError == AwmError.Ok && result.HasValue)
        {
            return result.Value.GetTag();
        }

        return null;
    }

    /// <summary>
    /// Checks if detection can start.
    /// </summary>
    public bool CanDetect => InputFiles.Any() && !IsProcessing;
}
