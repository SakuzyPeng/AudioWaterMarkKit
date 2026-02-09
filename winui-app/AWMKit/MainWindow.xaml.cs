using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using AWMKit.ViewModels;
using AWMKit.Pages;

namespace AWMKit;

/// <summary>
/// Main window with top NavigationView.
/// </summary>
public sealed partial class MainWindow : Window
{
    public MainWindow()
    {
        InitializeComponent();
        InitializeWindow();
        _ = InitializeAsync();
    }

    private async System.Threading.Tasks.Task InitializeAsync()
    {
        await AppViewModel.Instance.InitializeAsync();
        NavigateToEmbed();
    }

    private void InitializeWindow()
    {
        // Keep native WinUI title bar and window controls.
        Title = "AWMKit";
    }

    private void NavigateToEmbed()
    {
        MainNavigation.SelectedItem = MainNavigation.MenuItems[0];
        ContentFrame.Navigate(typeof(EmbedPage));
    }

    private void MainNavigation_SelectionChanged(NavigationView sender, NavigationViewSelectionChangedEventArgs args)
    {
        if (args.SelectedItemContainer is NavigationViewItem item)
        {
            var tag = item.Tag?.ToString();
            switch (tag)
            {
                case "embed":
                    ContentFrame.Navigate(typeof(EmbedPage));
                    break;
                case "detect":
                    ContentFrame.Navigate(typeof(DetectPage));
                    break;
                case "tags":
                    ContentFrame.Navigate(typeof(TagsPage));
                    break;
            }
        }
    }

}
