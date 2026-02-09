using AWMKit.Models;
using AWMKit.Native;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using System;
using System.Collections.ObjectModel;
using System.IO;
using System.Linq;
using System.Threading.Tasks;

namespace AWMKit.ViewModels;

/// <summary>
/// View model for the Embed page.
/// </summary>
public sealed partial class EmbedViewModel : ObservableObject
{
    [ObservableProperty]
    private string _identity = string.Empty;

    [ObservableProperty]
    private string _outputDirectory = string.Empty;

    [ObservableProperty]
    private int _strength = 10;

    [ObservableProperty]
    private bool _overwrite;

    [ObservableProperty]
    private bool _isProcessing;

    [ObservableProperty]
    private int _processedCount;

    [ObservableProperty]
    private int _totalCount;

    [ObservableProperty]
    private string? _currentFile;

    [ObservableProperty]
    private string? _errorMessage;

    public ObservableCollection<string> InputFiles { get; } = new();
    public ObservableCollection<string> ProcessedFiles { get; } = new();

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
    /// Clears all input files.
    /// </summary>
    [RelayCommand]
    private void ClearFiles()
    {
        InputFiles.Clear();
        ProcessedFiles.Clear();
        ErrorMessage = null;
    }

    /// <summary>
    /// Starts the embedding process.
    /// </summary>
    [RelayCommand]
    private async Task EmbedAsync()
    {
        if (string.IsNullOrEmpty(Identity) || !InputFiles.Any() || string.IsNullOrEmpty(OutputDirectory))
        {
            ErrorMessage = "Please fill in all required fields";
            return;
        }

        IsProcessing = true;
        ProcessedCount = 0;
        TotalCount = InputFiles.Count;
        ProcessedFiles.Clear();
        ErrorMessage = null;

        await Task.Run(async () =>
        {
            // Load or generate global key
            var (key, isNew, keyError) = AwmKeyBridge.GetOrCreateKey();
            if (key is null || keyError != AwmError.Ok)
            {
                ErrorMessage = $"Key error: {keyError}";
                IsProcessing = false;
                return;
            }

            // Create tag from identity
            var (tag, tagError) = AwmBridge.CreateTag(Identity);
            if (tag is null || tagError != AwmError.Ok)
            {
                ErrorMessage = $"Tag creation error: {tagError}";
                IsProcessing = false;
                return;
            }

            // Encode message
            var (message, encodeError) = AwmBridge.EncodeMessage(tag, key);
            if (encodeError != AwmError.Ok)
            {
                ErrorMessage = $"Message encoding error: {encodeError}";
                IsProcessing = false;
                return;
            }

            // Process each file
            foreach (var inputFile in InputFiles.ToList())
            {
                CurrentFile = Path.GetFileName(inputFile);

                var outputFile = Path.Combine(OutputDirectory, $"wm_{Path.GetFileName(inputFile)}");

                // Check overwrite
                if (!Overwrite && File.Exists(outputFile))
                {
                    ProcessedCount++;
                    continue;
                }

                // Embed watermark
                var embedError = AwmBridge.EmbedAudio(inputFile, outputFile, message, Strength);

                if (embedError == AwmError.Ok)
                {
                    ProcessedFiles.Add(outputFile);
                }
                else
                {
                    ErrorMessage = $"Failed to embed {Path.GetFileName(inputFile)}: {embedError}";
                }

                ProcessedCount++;
            }

            CurrentFile = null;
            IsProcessing = false;

            // Refresh app stats
            await AppViewModel.Instance.RefreshStatsAsync();
        });
    }

    /// <summary>
    /// Checks if embedding can start.
    /// </summary>
    public bool CanEmbed => !string.IsNullOrEmpty(Identity) && InputFiles.Any() && !string.IsNullOrEmpty(OutputDirectory) && !IsProcessing;
}
