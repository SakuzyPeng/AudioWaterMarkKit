using Microsoft.Data.Sqlite;
using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Threading.Tasks;

namespace AWMKit.Data;

/// <summary>
/// SQLite database manager for AWMKit.
/// Database location: %LOCALAPPDATA%\awmkit\awmkit.db
/// </summary>
public sealed class AppDatabase : IDisposable
{
    private static readonly string[] TagMappingsColumns =
    [
        "username",
        "tag",
        "created_at"
    ];

    private static readonly string[] AudioEvidenceColumns =
    [
        "id",
        "created_at",
        "file_path",
        "tag",
        "identity",
        "version",
        "key_slot",
        "timestamp_minutes",
        "message_hex",
        "sample_rate",
        "channels",
        "sample_count",
        "pcm_sha256",
        "chromaprint_blob",
        "fingerprint_len",
        "fp_config_id"
    ];

    private const string CreateTagMappingsSql = """
        CREATE TABLE IF NOT EXISTS tag_mappings (
            username TEXT NOT NULL COLLATE NOCASE PRIMARY KEY,
            tag TEXT NOT NULL,
            created_at INTEGER NOT NULL
        );
        """;

    private const string CreateAudioEvidenceSql = """
        CREATE TABLE IF NOT EXISTS audio_evidence (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            created_at INTEGER NOT NULL,
            file_path TEXT NOT NULL,
            tag TEXT NOT NULL,
            identity TEXT NOT NULL,
            version INTEGER NOT NULL,
            key_slot INTEGER NOT NULL,
            timestamp_minutes INTEGER NOT NULL,
            message_hex TEXT NOT NULL,
            sample_rate INTEGER NOT NULL,
            channels INTEGER NOT NULL,
            sample_count INTEGER NOT NULL,
            pcm_sha256 TEXT NOT NULL,
            chromaprint_blob BLOB NOT NULL,
            fingerprint_len INTEGER NOT NULL,
            fp_config_id INTEGER NOT NULL,
            UNIQUE(identity, key_slot, pcm_sha256)
        );
        """;

    private const string CreateTagMappingsIndexSql = """
        CREATE INDEX IF NOT EXISTS idx_tag_mappings_created_at
        ON tag_mappings(created_at DESC);
        """;

    private const string CreateAudioEvidenceIndexSql = """
        CREATE INDEX IF NOT EXISTS idx_audio_evidence_identity_slot_created
        ON audio_evidence(identity, key_slot, created_at DESC);
        """;

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

        await EnsureTableSchemaAsync("tag_mappings", TagMappingsColumns, CreateTagMappingsSql);
        await EnsureTableSchemaAsync("audio_evidence", AudioEvidenceColumns, CreateAudioEvidenceSql);

        using var cmd = _connection.CreateCommand();
        cmd.CommandText = CreateTagMappingsIndexSql + CreateAudioEvidenceIndexSql;
        await cmd.ExecuteNonQueryAsync();
    }

    private async Task EnsureTableSchemaAsync(string tableName, IReadOnlyList<string> expectedColumns, string createSql)
    {
        if (_connection is null)
        {
            return;
        }

        var existingColumns = await GetTableColumnsAsync(tableName);
        if (existingColumns.Count > 0 && !ColumnsMatch(existingColumns, expectedColumns))
        {
            using var dropCmd = _connection.CreateCommand();
            dropCmd.CommandText = $"DROP TABLE IF EXISTS {tableName}";
            await dropCmd.ExecuteNonQueryAsync();
            existingColumns.Clear();
        }

        if (existingColumns.Count == 0)
        {
            using var createCmd = _connection.CreateCommand();
            createCmd.CommandText = createSql;
            await createCmd.ExecuteNonQueryAsync();
        }
    }

    private async Task<List<string>> GetTableColumnsAsync(string tableName)
    {
        var columns = new List<string>();
        if (_connection is null)
        {
            return columns;
        }

        using var cmd = _connection.CreateCommand();
        cmd.CommandText = $"PRAGMA table_info({tableName})";
        using var reader = await cmd.ExecuteReaderAsync();
        while (await reader.ReadAsync())
        {
            columns.Add(reader.GetString(1));
        }
        return columns;
    }

    private static bool ColumnsMatch(IReadOnlyList<string> existing, IReadOnlyList<string> expected)
    {
        if (existing.Count != expected.Count)
        {
            return false;
        }

        return existing
            .Zip(expected, (left, right) => string.Equals(left, right, StringComparison.OrdinalIgnoreCase))
            .All(match => match);
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
