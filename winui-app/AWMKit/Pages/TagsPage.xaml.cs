using AWMKit.Models;
using AWMKit.ViewModels;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;

namespace AWMKit.Pages;

/// <summary>
/// Tag management page.
/// </summary>
public sealed partial class TagsPage : Page
{
    public TagsViewModel ViewModel { get; }

    public TagsPage()
    {
        InitializeComponent();
        ViewModel = new TagsViewModel();
    }

    private async void Page_Loaded(object sender, RoutedEventArgs e)
    {
        await ViewModel.LoadMappingsAsync();
    }

    private async void DeleteMappingButton_Click(object sender, RoutedEventArgs e)
    {
        if (sender is Button button && button.Tag is TagMapping mapping)
        {
            var dialog = new ContentDialog
            {
                Title = "Delete Tag Mapping",
                Content = $"Delete mapping for '{mapping.Identity}'?\nThis will also delete all associated evidence records.",
                PrimaryButtonText = "Delete",
                CloseButtonText = "Cancel",
                DefaultButton = ContentDialogButton.Close,
                XamlRoot = XamlRoot
            };

            var result = await dialog.ShowAsync();
            if (result == ContentDialogResult.Primary)
            {
                await ViewModel.DeleteMappingAsync(mapping);
            }
        }
    }

    private async void DeleteEvidenceButton_Click(object sender, RoutedEventArgs e)
    {
        if (sender is Button button && button.Tag is TagMapping mapping)
        {
            var dialog = new ContentDialog
            {
                Title = "Delete Evidence",
                Content = $"Delete all evidence records for tag '{mapping.Tag}'?",
                PrimaryButtonText = "Delete",
                CloseButtonText = "Cancel",
                DefaultButton = ContentDialogButton.Close,
                XamlRoot = XamlRoot
            };

            var result = await dialog.ShowAsync();
            if (result == ContentDialogResult.Primary)
            {
                await ViewModel.DeleteEvidenceAsync(mapping);
            }
        }
    }
}
