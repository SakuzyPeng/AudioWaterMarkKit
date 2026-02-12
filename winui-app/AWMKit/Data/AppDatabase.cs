using System;
using System.IO;
using System.Threading.Tasks;
using Microsoft.Data.Sqlite;

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
    /// Ensures baseline schema exists for tools/tests that access sqlite directly.
    /// </summary>
    public async Task<bool> OpenAsync()
    {
        try
        {
            await using var conn = new SqliteConnection($"Data Source={_databasePath}");
            await conn.OpenAsync();

            await using var cmd = conn.CreateCommand();
            cmd.CommandText = """
                CREATE TABLE IF NOT EXISTS tag_mappings (
                    username TEXT NOT NULL COLLATE NOCASE PRIMARY KEY,
                    tag TEXT NOT NULL,
                    created_at INTEGER NOT NULL
                );
                CREATE INDEX IF NOT EXISTS idx_tag_mappings_created_at
                ON tag_mappings(created_at DESC);

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
                    key_id TEXT NOT NULL,
                    is_forced_embed INTEGER NOT NULL DEFAULT 0,
                    snr_db REAL NULL,
                    snr_status TEXT NOT NULL DEFAULT 'unavailable',
                    chromaprint_blob BLOB NOT NULL,
                    fingerprint_len INTEGER NOT NULL,
                    fp_config_id INTEGER NOT NULL,
                    UNIQUE(identity, key_slot, key_id, pcm_sha256)
                );
                CREATE INDEX IF NOT EXISTS idx_audio_evidence_identity_slot_created
                ON audio_evidence(identity, key_slot, created_at DESC);
                CREATE INDEX IF NOT EXISTS idx_audio_evidence_slot_key_created
                ON audio_evidence(key_slot, key_id, created_at DESC);
                """;
            await cmd.ExecuteNonQueryAsync();

            _isOpen = true;
            return true;
        }
        catch
        {
            _isOpen = false;
            return false;
        }
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
