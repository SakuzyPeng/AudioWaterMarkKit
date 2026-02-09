using Microsoft.UI;
using Microsoft.UI.Composition.SystemBackdrops;
using Microsoft.UI.Windowing;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Media;
using System;
using System.Runtime.InteropServices;
using WinRT.Interop;
using AWMKit.Native;
using AWMKit.ViewModels;
using AWMKit.Pages;

namespace AWMKit;

/// <summary>
/// Main window with NavigationView, custom title bar, and status indicators.
/// </summary>
public sealed partial class MainWindow : Window
{
    private AppWindow? _appWindow;

    public MainWindow()
    {
        InitializeComponent();
        InitializeWindow();
        _ = InitializeAsync();
    }

    private async System.Threading.Tasks.Task InitializeAsync()
    {
        await AppViewModel.Instance.InitializeAsync();
        InitializeStatusIndicators();
        NavigateToEmbed();
    }

    private void InitializeWindow()
    {
        // Get AppWindow for title bar customization
        var hWnd = WindowNative.GetWindowHandle(this);
        var windowId = Win32Interop.GetWindowIdFromWindow(hWnd);
        _appWindow = AppWindow.GetFromWindowId(windowId);

        // Set window size and title
        _appWindow.Resize(new Windows.Graphics.SizeInt32(900, 700));
        Title = "AWMKit";

        // Enable Mica backdrop
        if (MicaController.IsSupported())
        {
            SystemBackdrop = new MicaBackdrop { Kind = MicaKind.Base };
        }

        // Customize title bar
        if (AppWindowTitleBar.IsCustomizationSupported())
        {
            var titleBar = _appWindow.TitleBar;
            titleBar.ExtendsContentIntoTitleBar = true;
            titleBar.ButtonBackgroundColor = Colors.Transparent;
            titleBar.ButtonInactiveBackgroundColor = Colors.Transparent;

            // Set title bar drag region
            SetTitleBar(AppTitleBar);
        }
    }

    private void InitializeStatusIndicators()
    {
        UpdateKeyStatus(AppViewModel.Instance.KeyAvailable);
        UpdateEngineStatus(AppViewModel.Instance.EngineAvailable);
        UpdateDatabaseStatus(AppViewModel.Instance.DatabaseAvailable);
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

    /// <summary>
    /// Updates the key status indicator.
    /// </summary>
    public void UpdateKeyStatus(bool available)
    {
        DispatcherQueue.TryEnqueue(() =>
        {
            KeyStatusBadge.Opacity = available ? 1.0 : 0.4;
            KeyStatusBadge.Background = available
                ? new SolidColorBrush(Windows.UI.Color.FromArgb(255, 76, 175, 80))  // Green
                : new SolidColorBrush(Windows.UI.Color.FromArgb(255, 158, 158, 158)); // Gray
        });
    }

    /// <summary>
    /// Updates the engine status indicator.
    /// </summary>
    public void UpdateEngineStatus(bool available)
    {
        DispatcherQueue.TryEnqueue(() =>
        {
            EngineStatusBadge.Opacity = available ? 1.0 : 0.4;
            EngineStatusBadge.Background = available
                ? new SolidColorBrush(Windows.UI.Color.FromArgb(255, 33, 150, 243))  // Blue
                : new SolidColorBrush(Windows.UI.Color.FromArgb(255, 158, 158, 158)); // Gray
        });
    }

    /// <summary>
    /// Updates the database status indicator.
    /// </summary>
    public void UpdateDatabaseStatus(bool available)
    {
        DispatcherQueue.TryEnqueue(() =>
        {
            DatabaseStatusBadge.Opacity = available ? 1.0 : 0.4;
            DatabaseStatusBadge.Background = available
                ? new SolidColorBrush(Windows.UI.Color.FromArgb(255, 255, 152, 0))   // Orange
                : new SolidColorBrush(Windows.UI.Color.FromArgb(255, 158, 158, 158)); // Gray
        });
    }
}
