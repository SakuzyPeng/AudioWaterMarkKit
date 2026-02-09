using AWMKit.ViewModels;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using System;
using System.Linq;
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

    public EmbedPage()
    {
        InitializeComponent();
        ViewModel = new EmbedViewModel();
    }

    private async void AddFilesButton_Click(object sender, RoutedEventArgs e)
    {
        var picker = new FileOpenPicker();
        picker.FileTypeFilter.Add(".wav");
        picker.FileTypeFilter.Add(".mp3");
        picker.FileTypeFilter.Add(".flac");
        picker.FileTypeFilter.Add(".m4a");

        var hWnd = WindowNative.GetWindowHandle(App.Current.MainWindow);
        InitializeWithWindow.Initialize(picker, hWnd);

        var files = await picker.PickMultipleFilesAsync();
        if (files.Count > 0)
        {
            ViewModel.AddFilesCommand.Execute(files.Select(f => f.Path).ToArray());
        }
    }

    private async void BrowseOutputButton_Click(object sender, RoutedEventArgs e)
    {
        var picker = new FolderPicker();
        picker.FileTypeFilter.Add("*");

        var hWnd = WindowNative.GetWindowHandle(App.Current.MainWindow);
        InitializeWithWindow.Initialize(picker, hWnd);

        var folder = await picker.PickSingleFolderAsync();
        if (folder is not null)
        {
            ViewModel.OutputDirectory = folder.Path;
        }
    }

    private void RemoveFileButton_Click(object sender, RoutedEventArgs e)
    {
        if (sender is Button button && button.Tag is string filePath)
        {
            ViewModel.RemoveFileCommand.Execute(filePath);
        }
    }
}
