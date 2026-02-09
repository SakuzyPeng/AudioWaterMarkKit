using Microsoft.UI.Xaml;
using System;

namespace AWMKit;

/// <summary>
/// AWMKit WinUI 3 application entry point.
/// </summary>
public partial class App : Application
{
    private Window? _mainWindow;

    public App()
    {
        InitializeComponent();
    }

    /// <summary>
    /// Invoked when the application is launched.
    /// </summary>
    protected override void OnLaunched(LaunchActivatedEventArgs args)
    {
        _mainWindow = new MainWindow();
        _mainWindow.Activate();
    }

    /// <summary>
    /// Gets the current application instance.
    /// </summary>
    public static new App Current => (App)Application.Current;

    /// <summary>
    /// Gets the main window instance.
    /// </summary>
    public Window? MainWindow => _mainWindow;
}
