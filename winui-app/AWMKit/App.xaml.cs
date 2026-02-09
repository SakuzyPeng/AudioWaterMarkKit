using Microsoft.UI.Xaml;
using System;
using System.IO;
using System.Threading.Tasks;

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
        UnhandledException += OnUnhandledException;
        AppDomain.CurrentDomain.UnhandledException += OnCurrentDomainUnhandledException;
        TaskScheduler.UnobservedTaskException += OnUnobservedTaskException;
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

    private static void OnUnhandledException(object sender, Microsoft.UI.Xaml.UnhandledExceptionEventArgs e)
    {
        WriteCrashLog("WinUI.UnhandledException", e.Exception);
    }

    private static void OnCurrentDomainUnhandledException(object? sender, System.UnhandledExceptionEventArgs e)
    {
        WriteCrashLog("AppDomain.UnhandledException", e.ExceptionObject as Exception);
    }

    private static void OnUnobservedTaskException(object? sender, UnobservedTaskExceptionEventArgs e)
    {
        WriteCrashLog("TaskScheduler.UnobservedTaskException", e.Exception);
    }

    private static void WriteCrashLog(string source, Exception? ex)
    {
        try
        {
            var dir = Path.Combine(
                Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData),
                "awmkit");
            Directory.CreateDirectory(dir);
            var path = Path.Combine(dir, "winui-crash.log");
            var payload = $"[{DateTimeOffset.Now:yyyy-MM-dd HH:mm:ss}] {source}{Environment.NewLine}{ex}{Environment.NewLine}{Environment.NewLine}";
            File.AppendAllText(path, payload);
        }
        catch
        {
            // Ignore logging failure.
        }
    }
}
