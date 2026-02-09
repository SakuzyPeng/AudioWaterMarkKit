using Microsoft.Data.Sqlite;
using System;
using System.IO;
using System.Threading.Tasks;

namespace AWMKit.Data;

/// <summary>
/// SQLite database manager for AWMKit.
/// Database location: %LOCALAPPDATA%\awmkit\awmkit.db
/// </summary>
public sealed class AppDatabase : IDisposable
{
    private readonly string _databasePath;
    private SqliteConnection? _connection;
    private bool _disposed;

    public AppDatabase()
    {
        var appDataPath = Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData);
        var awmkitDir = Path.Combine(appDataPath, "awmkit");
        Directory.CreateDirectory(awmkitDir);
        _databasePath = Path.Combine(awmkitDir, "awmkit.db");
    }

    /// <summary>
    /// Gets the database file path.
    /// </summary>
    public string DatabasePath => _databasePath;

    /// <summary>
    /// Opens the database connection and initializes schema if needed.
    /// </summary>
    public async Task<bool> OpenAsync()
    {
        if (_connection is not null)
        {
            return true;
        }

        try
        {
            _connection = new SqliteConnection($"Data Source={_databasePath}");
            await _connection.OpenAsync();
            await InitializeSchemaAsync();
            return true;
        }
        catch
        {
            _connection?.Dispose();
            _connection = null;
            return false;
        }
    }

    /// <summary>
    /// Checks if the database connection is open.
    /// </summary>
    public bool IsOpen => _connection?.State == System.Data.ConnectionState.Open;

    /// <summary>
    /// Gets the active database connection.
    /// </summary>
    public SqliteConnection? Connection => _connection;

    private async Task InitializeSchemaAsync()
    {
        if (_connection is null)
        {
            return;
        }

        // Create tag_mappings table (shared schema with Rust side)
        var createTagMappings = @"
CREATE TABLE IF NOT EXISTS tag_mappings (
    identity TEXT PRIMARY KEY NOT NULL,
    tag TEXT NOT NULL UNIQUE,
    display_name TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
)";

        // Create audio_evidence table (shared schema with Rust side)
        var createAudioEvidence = @"
CREATE TABLE IF NOT EXISTS audio_evidence (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    file_path TEXT NOT NULL,
    file_hash TEXT NOT NULL UNIQUE,
    message TEXT NOT NULL,
    pattern TEXT NOT NULL,
    tag TEXT NOT NULL,
    created_at TEXT NOT NULL
)";

        // Create indices for performance
        var createIndices = @"
CREATE INDEX IF NOT EXISTS idx_tag_mappings_tag ON tag_mappings(tag);
CREATE INDEX IF NOT EXISTS idx_audio_evidence_tag ON audio_evidence(tag);
CREATE INDEX IF NOT EXISTS idx_audio_evidence_hash ON audio_evidence(file_hash);
";

        using var cmd = _connection.CreateCommand();
        cmd.CommandText = createTagMappings;
        await cmd.ExecuteNonQueryAsync();

        cmd.CommandText = createAudioEvidence;
        await cmd.ExecuteNonQueryAsync();

        cmd.CommandText = createIndices;
        await cmd.ExecuteNonQueryAsync();
    }

    /// <summary>
    /// Checks if the database file exists and is accessible.
    /// </summary>
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

        _connection?.Dispose();
        _connection = null;
        _disposed = true;
    }
}
