using AWMKit.ViewModels;
using AWMKit.Models;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Input;
using System;
using System.Collections.Generic;
using System.ComponentModel;
using System.Linq;
using System.Threading.Tasks;
using Windows.ApplicationModel.DataTransfer;
using Windows.Storage;
using Windows.Storage.Pickers;
using WinRT.Interop;

namespace AWMKit.Pages;

/// <summary>
/// Detect watermark page.
/// </summary>
public sealed partial class DetectPage : Page
{
    public DetectViewModel ViewModel { get; }
    public AppViewModel AppState { get; } = AppViewModel.Instance;

    public DetectPage()
    {
        InitializeComponent();
        ViewModel = new DetectViewModel();
        ViewModel.IsKeyAvailable = AppState.KeyAvailable;
        AppState.PropertyChanged += AppStateOnPropertyChanged;
        Loaded += DetectPage_Loaded;
        Unloaded += DetectPage_Unloaded;
    }

    private async void DetectPage_Loaded(object sender, RoutedEventArgs e)
    {
        await AppState.RefreshRuntimeStatusAsync();
        ViewModel.IsKeyAvailable = AppState.KeyAvailable;
    }

    private void DetectPage_Unloaded(object sender, RoutedEventArgs e)
    {
        AppState.PropertyChanged -= AppStateOnPropertyChanged;
        Unloaded -= DetectPage_Unloaded;
        Loaded -= DetectPage_Loaded;
    }

    private void AppStateOnPropertyChanged(object? sender, PropertyChangedEventArgs e)
    {
        if (e.PropertyName == nameof(AppViewModel.KeyAvailable))
        {
            _ = DispatcherQueue.TryEnqueue(() => { ViewModel.IsKeyAvailable = AppState.KeyAvailable; });
        }
    }

    private async void SelectInputSourceButton_Click(object sender, RoutedEventArgs e)
    {
        var dialog = new ContentDialog
        {
            Title = "选择输入源",
            Content = "请选择输入类型",
            PrimaryButtonText = "文件",
            SecondaryButtonText = "目录",
            CloseButtonText = "取消",
            DefaultButton = ContentDialogButton.Primary,
            XamlRoot = XamlRoot,
        };

        var result = await dialog.ShowAsync();
        switch (result)
        {
            case ContentDialogResult.Primary:
            {
                var path = await PickSingleAudioFileAsync();
                if (!string.IsNullOrWhiteSpace(path))
                {
                    ViewModel.SetInputSource(path);
                }
                break;
            }
            case ContentDialogResult.Secondary:
            {
                var path = await PickFolderAsync();
                if (!string.IsNullOrWhiteSpace(path))
                {
                    ViewModel.SetInputSource(path);
                }
                break;
            }
        }
    }

    private async Task<string?> PickSingleAudioFileAsync()
    {
        var picker = new FileOpenPicker();
        picker.FileTypeFilter.Add(".wav");
        picker.FileTypeFilter.Add(".flac");
        picker.FileTypeFilter.Add(".m4a");
        picker.FileTypeFilter.Add(".alac");

        var hWnd = WindowNative.GetWindowHandle(App.Current.MainWindow);
        InitializeWithWindow.Initialize(picker, hWnd);

        var file = await picker.PickSingleFileAsync();
        return file?.Path;
    }

    private void GoToKeyPageButton_Click(object sender, RoutedEventArgs e)
    {
        if (App.Current.MainWindow is AWMKit.MainWindow window)
        {
            window.NavigateToKeyPage();
        }
    }

    private async Task<string?> PickFolderAsync()
    {
        var picker = new FolderPicker();
        picker.FileTypeFilter.Add("*");

        var hWnd = WindowNative.GetWindowHandle(App.Current.MainWindow);
        InitializeWithWindow.Initialize(picker, hWnd);

        var folder = await picker.PickSingleFolderAsync();
        return folder?.Path;
    }

    private void RemoveQueueFileButton_Click(object sender, RoutedEventArgs e)
    {
        if (sender is Button button && button.Tag is string filePath)
        {
            ViewModel.RemoveQueueFileCommand.Execute(filePath);
        }
    }

    private void LogEntryButton_Click(object sender, RoutedEventArgs e)
    {
        if (sender is Button button && button.Tag is LogEntry entry)
        {
            ViewModel.ToggleLogSelectionCommand.Execute(entry);
        }
    }

    private void DropZone_DragOver(object sender, DragEventArgs e)
    {
        if (e.DataView.Contains(StandardDataFormats.StorageItems))
        {
            e.AcceptedOperation = DataPackageOperation.Copy;
        }
        else
        {
            e.AcceptedOperation = DataPackageOperation.None;
        }

        e.DragUIOverride.Caption = "拖拽到此处添加到队列";
        e.DragUIOverride.IsCaptionVisible = true;
    }

    private async void DropZone_Drop(object sender, DragEventArgs e)
    {
        if (!e.DataView.Contains(StandardDataFormats.StorageItems))
        {
            return;
        }

        var items = await e.DataView.GetStorageItemsAsync();
        var droppedFiles = new List<string>();

        foreach (var item in items)
        {
            if (item is StorageFile file)
            {
                droppedFiles.Add(file.Path);
            }
            else if (item is StorageFolder folder)
            {
                var files = await folder.GetFilesAsync();
                droppedFiles.AddRange(files.Select(x => x.Path));
            }
        }

        ViewModel.AddDroppedFiles(droppedFiles);
    }
}
