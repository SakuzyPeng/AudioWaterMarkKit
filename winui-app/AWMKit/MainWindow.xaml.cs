using AWMKit.Pages;
using AWMKit.ViewModels;
using Microsoft.UI.Windowing;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Automation;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Media;
using System;
using System.ComponentModel;
using System.IO;
using WinRT.Interop;

namespace AWMKit;

/// <summary>
/// Main window with top NavigationView.
/// </summary>
public sealed partial class MainWindow : Window
{
    public AppViewModel ViewModel { get; } = AppViewModel.Instance;

    private NavigationViewItem? _lastPageItem;
    private NavigationViewItem? _keyStatusItem;
    private NavigationViewItem? _engineStatusItem;
    private NavigationViewItem? _databaseStatusItem;

    public MainWindow()
    {
        InitializeComponent();
        InitializeStatusIndicators();
        InitializeWindow();
        ViewModel.PropertyChanged += ViewModel_PropertyChanged;
        _ = InitializeAsync();
    }

    private async System.Threading.Tasks.Task InitializeAsync()
    {
        await ViewModel.InitializeAsync();
        UpdateStatusIndicators();
        NavigateToEmbed();
    }

    private void InitializeWindow()
    {
        // Keep native WinUI title bar and window controls.
        Title = "AWMKit";
        TrySetWindowIcon();
    }

    private void TrySetWindowIcon()
    {
        try
        {
            var iconPath = Path.Combine(AppContext.BaseDirectory, "Assets", "AppIcon.ico");
            if (!File.Exists(iconPath))
            {
                return;
            }

            var hwnd = WindowNative.GetWindowHandle(this);
            var windowId = Microsoft.UI.Win32Interop.GetWindowIdFromWindow(hwnd);
            var appWindow = AppWindow.GetFromWindowId(windowId);
            appWindow.SetIcon(iconPath);
        }
        catch
        {
            // Ignore icon setup failure.
        }
    }

    private void InitializeStatusIndicators()
    {
        _keyStatusItem = CreateStatusItem("status:key", Symbol.Permissions, "密钥状态");
        _engineStatusItem = CreateStatusItem("status:engine", Symbol.Audio, "音频引擎状态");
        _databaseStatusItem = CreateStatusItem("status:database", Symbol.Library, "数据库状态");

        MainNavigation.FooterMenuItems.Clear();
        MainNavigation.FooterMenuItems.Add(_keyStatusItem);
        MainNavigation.FooterMenuItems.Add(_engineStatusItem);
        MainNavigation.FooterMenuItems.Add(_databaseStatusItem);
        UpdateStatusIndicators();
    }

    private static NavigationViewItem CreateStatusItem(string tag, Symbol symbol, string automationName)
    {
        var icon = new SymbolIcon(symbol)
        {
            HorizontalAlignment = HorizontalAlignment.Center,
            VerticalAlignment = VerticalAlignment.Center
        };

        var item = new NavigationViewItem
        {
            Tag = tag,
            Content = null,
            Icon = icon,
            Width = 40,
            Height = 32,
            MinWidth = 40,
            Padding = new Thickness(0),
            HorizontalContentAlignment = HorizontalAlignment.Center,
            VerticalContentAlignment = VerticalAlignment.Center,
            HorizontalAlignment = HorizontalAlignment.Center,
            VerticalAlignment = VerticalAlignment.Center,
            Margin = new Thickness(0, 0, 4, 0)
        };
        AutomationProperties.SetName(item, automationName);
        return item;
    }

    private void UpdateStatusIndicators()
    {
        UpdateStatusItem(_keyStatusItem, ViewModel.KeyStatusBrush, ViewModel.KeyStatusTooltip);
        UpdateStatusItem(_engineStatusItem, ViewModel.EngineStatusBrush, ViewModel.EngineStatusTooltip);
        UpdateStatusItem(_databaseStatusItem, ViewModel.DatabaseStatusBrush, ViewModel.DatabaseStatusTooltip);
    }

    private static void UpdateStatusItem(NavigationViewItem? item, Brush brush, string tooltip)
    {
        if (item is null)
        {
            return;
        }

        if (item.Icon is IconElement icon)
        {
            icon.Foreground = brush;
        }

        ToolTipService.SetToolTip(item, tooltip);
    }

    private void ViewModel_PropertyChanged(object? sender, PropertyChangedEventArgs e)
    {
        _ = DispatcherQueue.TryEnqueue(UpdateStatusIndicators);
    }

    private void NavigateToEmbed()
    {
        if (MainNavigation.MenuItems.Count == 0)
        {
            return;
        }

        if (MainNavigation.MenuItems[0] is NavigationViewItem embedItem)
        {
            _lastPageItem = embedItem;
            MainNavigation.SelectedItem = embedItem;
        }
        ContentFrame.Navigate(typeof(EmbedPage));
    }

    private void MainNavigation_SelectionChanged(NavigationView sender, NavigationViewSelectionChangedEventArgs args)
    {
        if (args.SelectedItemContainer is not NavigationViewItem item)
        {
            return;
        }

        var tag = item.Tag?.ToString();
        switch (tag)
        {
            case "embed":
                _lastPageItem = item;
                ContentFrame.Navigate(typeof(EmbedPage));
                break;
            case "detect":
                _lastPageItem = item;
                ContentFrame.Navigate(typeof(DetectPage));
                break;
            case "key":
                _lastPageItem = item;
                ContentFrame.Navigate(typeof(KeyPage));
                break;
            case "tags":
                _lastPageItem = item;
                ContentFrame.Navigate(typeof(TagsPage));
                break;
            case "status:key":
            case "status:engine":
            case "status:database":
                _ = ViewModel.RefreshRuntimeStatusAsync();
                if (_lastPageItem is not null)
                {
                    MainNavigation.SelectedItem = _lastPageItem;
                }
                break;
        }
    }

    public void NavigateToKeyPage()
    {
        foreach (var menuItem in MainNavigation.MenuItems)
        {
            if (menuItem is NavigationViewItem item && item.Tag?.ToString() == "key")
            {
                MainNavigation.SelectedItem = item;
                return;
            }
        }
    }
}
