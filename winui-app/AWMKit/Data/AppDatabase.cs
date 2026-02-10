using System;
using System.IO;
using System.Threading.Tasks;

namespace AWMKit.Data;

/// <summary>
/// Database path/container for shared awmkit.sqlite.
/// SQL CRUD is handled by Rust via FFI.
/// </summary>
public sealed class AppDatabase : IDisposable
{
    private readonly string _databasePath;
    private bool _isOpen;
    private bool _disposed;

    public AppDatabase()
    {
        var appDataPath = Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData);
        var awmkitDir = Path.Combine(appDataPath, "awmkit");
        Directory.CreateDirectory(awmkitDir);
        _databasePath = Path.Combine(awmkitDir, "awmkit.db");
    }

    /// <summary>
    /// Gets the shared database file path.
    /// </summary>
    public string DatabasePath => _databasePath;

    /// <summary>
    /// Keeps compatibility with existing init flow.
    /// </summary>
    public Task<bool> OpenAsync()
    {
        _isOpen = true;
        return Task.FromResult(true);
    }

    public bool IsOpen => _isOpen;

    public static bool Exists()
    {
        var appDataPath = Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData);
        var dbPath = Path.Combine(appDataPath, "awmkit", "awmkit.db");
        return File.Exists(dbPath);
    }

    public void Dispose()
    {
        if (_disposed)
        {
            return;
        }

        _isOpen = false;
        _disposed = true;
    }
}
