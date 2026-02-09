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
    private string _identity = string.Empty;
    public string Identity
    {
        get => _identity;
        set => SetProperty(ref _identity, value);
    }

    private string _outputDirectory = string.Empty;
    public string OutputDirectory
    {
        get => _outputDirectory;
        set => SetProperty(ref _outputDirectory, value);
    }

    private int _strength = 10;
    public int Strength
    {
        get => _strength;
        set => SetProperty(ref _strength, value);
    }

    private bool _overwrite;
    public bool Overwrite
    {
        get => _overwrite;
        set => SetProperty(ref _overwrite, value);
    }

    private bool _isProcessing;
    public bool IsProcessing
    {
        get => _isProcessing;
        set => SetProperty(ref _isProcessing, value);
    }

    private int _processedCount;
    public int ProcessedCount
    {
        get => _processedCount;
        set => SetProperty(ref _processedCount, value);
    }

    private int _totalCount;
    public int TotalCount
    {
        get => _totalCount;
        set => SetProperty(ref _totalCount, value);
    }

    private string? _currentFile;
    public string? CurrentFile
    {
        get => _currentFile;
        set => SetProperty(ref _currentFile, value);
    }

    private string? _errorMessage;
    public string? ErrorMessage
    {
        get => _errorMessage;
        set => SetProperty(ref _errorMessage, value);
    }

    public ObservableCollection<ChannelLayoutOption> ChannelLayoutOptions { get; } = BuildChannelLayoutOptions();

    private ChannelLayoutOption? _selectedChannelLayout;
    public ChannelLayoutOption? SelectedChannelLayout
    {
        get => _selectedChannelLayout;
        set => SetProperty(ref _selectedChannelLayout, value);
    }

    public ObservableCollection<string> InputFiles { get; } = new();
    public ObservableCollection<string> ProcessedFiles { get; } = new();

    public EmbedViewModel()
    {
        SelectedChannelLayout = ChannelLayoutOptions.FirstOrDefault();
    }

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
            var (key, _, keyError) = AwmKeyBridge.GetOrCreateKey();
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
            if (encodeError != AwmError.Ok || message is null)
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
                var layout = SelectedChannelLayout?.Layout ?? AwmChannelLayout.Auto;
                var embedError = AwmBridge.EmbedAudioMultichannel(inputFile, outputFile, message, layout, Strength);

                if (embedError == AwmError.Ok)
                {
                    var recordError = AwmBridge.RecordEvidenceFile(outputFile, message, key);
                    if (recordError != AwmError.Ok && string.IsNullOrEmpty(ErrorMessage))
                    {
                        ErrorMessage = $"[WARN] Evidence record failed: {recordError}";
                    }

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
}
