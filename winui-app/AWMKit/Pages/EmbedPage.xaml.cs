using AWMKit.ViewModels;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using System.ComponentModel;
using System.Collections.Generic;
using System.Threading.Tasks;
using Windows.ApplicationModel.DataTransfer;
using Windows.Storage;
using Windows.Storage.Pickers;
using WinRT.Interop;

namespace AWMKit.Pages;

/// <summary>
/// Embed watermark page.
/// </summary>
public sealed partial class EmbedPage : Page
{
    public EmbedViewModel ViewModel { get; }
    public AppViewModel AppState { get; } = AppViewModel.Instance;

    public EmbedPage()
    {
        InitializeComponent();
        ViewModel = new EmbedViewModel();
        ViewModel.IsKeyAvailable = AppState.KeyAvailable;
        AppState.PropertyChanged += AppStateOnPropertyChanged;
        Unloaded += EmbedPage_Unloaded;
    }

    private async void Page_Loaded(object sender, RoutedEventArgs e)
    {
        await ViewModel.RefreshTagMappingsAsync();
        await AppState.RefreshRuntimeStatusAsync();
        ViewModel.IsKeyAvailable = AppState.KeyAvailable;
    }

    private void EmbedPage_Unloaded(object sender, RoutedEventArgs e)
    {
        AppState.PropertyChanged -= AppStateOnPropertyChanged;
        Unloaded -= EmbedPage_Unloaded;
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

    private async void SelectOutputDirectoryButton_Click(object sender, RoutedEventArgs e)
    {
        var path = await PickFolderAsync();
        if (!string.IsNullOrWhiteSpace(path))
        {
            ViewModel.OutputDirectory = path;
            await ViewModel.FlashOutputSelectAsync();
        }
    }

    private void GoToKeyPageButton_Click(object sender, RoutedEventArgs e)
    {
        if (App.Current.MainWindow is AWMKit.MainWindow window)
        {
            window.NavigateToKeyPage();
        }
    }

    private async Task<string?> PickSingleAudioFileAsync()
    {
        var picker = new FileOpenPicker();
        picker.FileTypeFilter.Add(".wav");
        picker.FileTypeFilter.Add(".flac");

        var hWnd = WindowNative.GetWindowHandle(App.Current.MainWindow);
        InitializeWithWindow.Initialize(picker, hWnd);

        var file = await picker.PickSingleFileAsync();
        return file?.Path;
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
        var dropped = new List<string>();

        foreach (var item in items)
        {
            if (item is StorageFile file)
            {
                dropped.Add(file.Path);
            }
            else if (item is StorageFolder folder)
            {
                var files = await folder.GetFilesAsync();
                foreach (var childFile in files)
                {
                    dropped.Add(childFile.Path);
                }
            }
        }

        ViewModel.AddDroppedFiles(dropped);
    }
}
