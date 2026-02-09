using AWMKit.Models;
using Microsoft.Data.Sqlite;
using System;
using System.Collections.Generic;
using System.Threading.Tasks;

namespace AWMKit.Data;

/// <summary>
/// Repository for audio_evidence table operations.
/// </summary>
public sealed class EvidenceStore
{
    private readonly AppDatabase _database;
    private const int DefaultVersion = 2;
    private const int DefaultKeySlot = 0;
    private const int DefaultFpConfigId = 1;

    public EvidenceStore(AppDatabase database)
    {
        _database = database;
    }

    /// <summary>
    /// Lists all evidence records.
    /// </summary>
    public async Task<List<EvidenceRecord>> ListAllAsync()
    {
        var records = new List<EvidenceRecord>();

        if (_database.Connection is null)
        {
            return records;
        }

        const string query = """
            SELECT id, created_at, file_path, tag, identity, version, key_slot, timestamp_minutes,
                   message_hex, sample_rate, channels, sample_count, pcm_sha256,
                   chromaprint_blob, fingerprint_len, fp_config_id
            FROM audio_evidence
            ORDER BY created_at DESC
            """;

        using var cmd = _database.Connection.CreateCommand();
        cmd.CommandText = query;

        using var reader = await cmd.ExecuteReaderAsync();
        while (await reader.ReadAsync())
        {
            records.Add(ReadRecord(reader));
        }

        return records;
    }

    /// <summary>
    /// Gets an evidence record by PCM SHA256 (legacy API name kept for compatibility).
    /// </summary>
    public async Task<EvidenceRecord?> GetByHashAsync(string fileHash)
    {
        if (_database.Connection is null)
        {
            return null;
        }

        const string query = """
            SELECT id, created_at, file_path, tag, identity, version, key_slot, timestamp_minutes,
                   message_hex, sample_rate, channels, sample_count, pcm_sha256,
                   chromaprint_blob, fingerprint_len, fp_config_id
            FROM audio_evidence
            WHERE pcm_sha256 = @pcmSha256
            LIMIT 1
            """;

        using var cmd = _database.Connection.CreateCommand();
        cmd.CommandText = query;
        cmd.Parameters.AddWithValue("@pcmSha256", fileHash);

        using var reader = await cmd.ExecuteReaderAsync();
        return await reader.ReadAsync() ? ReadRecord(reader) : null;
    }

    /// <summary>
    /// Lists evidence records by tag.
    /// </summary>
    public async Task<List<EvidenceRecord>> ListByTagAsync(string tag)
    {
        var records = new List<EvidenceRecord>();

        if (_database.Connection is null)
        {
            return records;
        }

        const string query = """
            SELECT id, created_at, file_path, tag, identity, version, key_slot, timestamp_minutes,
                   message_hex, sample_rate, channels, sample_count, pcm_sha256,
                   chromaprint_blob, fingerprint_len, fp_config_id
            FROM audio_evidence
            WHERE tag = @tag
            ORDER BY created_at DESC
            """;

        using var cmd = _database.Connection.CreateCommand();
        cmd.CommandText = query;
        cmd.Parameters.AddWithValue("@tag", tag);

        using var reader = await cmd.ExecuteReaderAsync();
        while (await reader.ReadAsync())
        {
            records.Add(ReadRecord(reader));
        }

        return records;
    }

    /// <summary>
    /// Saves an evidence record.
    /// This compatibility path fills missing Rust fields with safe defaults.
    /// </summary>
    public async Task<bool> SaveAsync(string filePath, string fileHash, string message, string pattern, string tag)
    {
        if (_database.Connection is null)
        {
            return false;
        }

        _ = pattern;
        var now = DateTimeOffset.UtcNow.ToUnixTimeSeconds();

        const string insert = """
            INSERT OR IGNORE INTO audio_evidence (
                created_at, file_path, tag, identity, version, key_slot, timestamp_minutes,
                message_hex, sample_rate, channels, sample_count, pcm_sha256,
                chromaprint_blob, fingerprint_len, fp_config_id
            ) VALUES (
                @createdAt, @filePath, @tag, @identity, @version, @keySlot, @timestampMinutes,
                @messageHex, @sampleRate, @channels, @sampleCount, @pcmSha256,
                @chromaprintBlob, @fingerprintLen, @fpConfigId
            )
            """;

        try
        {
            using var cmd = _database.Connection.CreateCommand();
            cmd.CommandText = insert;
            cmd.Parameters.AddWithValue("@createdAt", now);
            cmd.Parameters.AddWithValue("@filePath", filePath);
            cmd.Parameters.AddWithValue("@tag", tag);
            cmd.Parameters.AddWithValue("@identity", "UNKNOWN");
            cmd.Parameters.AddWithValue("@version", DefaultVersion);
            cmd.Parameters.AddWithValue("@keySlot", DefaultKeySlot);
            cmd.Parameters.AddWithValue("@timestampMinutes", 0);
            cmd.Parameters.AddWithValue("@messageHex", message);
            cmd.Parameters.AddWithValue("@sampleRate", 0);
            cmd.Parameters.AddWithValue("@channels", 0);
            cmd.Parameters.AddWithValue("@sampleCount", 0);
            cmd.Parameters.AddWithValue("@pcmSha256", fileHash);
            cmd.Parameters.AddWithValue("@chromaprintBlob", Array.Empty<byte>());
            cmd.Parameters.AddWithValue("@fingerprintLen", 0);
            cmd.Parameters.AddWithValue("@fpConfigId", DefaultFpConfigId);
            await cmd.ExecuteNonQueryAsync();
            return true;
        }
        catch (SqliteException)
        {
            return false;
        }
    }

    /// <summary>
    /// Deletes an evidence record by PCM SHA256 (legacy API name kept for compatibility).
    /// </summary>
    public async Task<bool> DeleteByHashAsync(string fileHash)
    {
        if (_database.Connection is null)
        {
            return false;
        }

        const string delete = "DELETE FROM audio_evidence WHERE pcm_sha256 = @pcmSha256";

        using var cmd = _database.Connection.CreateCommand();
        cmd.CommandText = delete;
        cmd.Parameters.AddWithValue("@pcmSha256", fileHash);

        var rowsAffected = await cmd.ExecuteNonQueryAsync();
        return rowsAffected > 0;
    }

    /// <summary>
    /// Deletes all evidence records for a given tag.
    /// </summary>
    public async Task<int> DeleteByTagAsync(string tag)
    {
        if (_database.Connection is null)
        {
            return 0;
        }

        const string delete = "DELETE FROM audio_evidence WHERE tag = @tag";

        using var cmd = _database.Connection.CreateCommand();
        cmd.CommandText = delete;
        cmd.Parameters.AddWithValue("@tag", tag);

        return await cmd.ExecuteNonQueryAsync();
    }

    /// <summary>
    /// Counts total evidence records.
    /// </summary>
    public async Task<int> CountAsync()
    {
        if (_database.Connection is null)
        {
            return 0;
        }

        const string query = "SELECT COUNT(*) FROM audio_evidence";

        using var cmd = _database.Connection.CreateCommand();
        cmd.CommandText = query;

        var result = await cmd.ExecuteScalarAsync();
        return result is long count ? (int)count : 0;
    }

    private static EvidenceRecord ReadRecord(SqliteDataReader reader)
    {
        return new EvidenceRecord
        {
            Id = reader.GetInt64(0),
            CreatedAt = reader.GetInt64(1),
            FilePath = reader.GetString(2),
            Tag = reader.GetString(3),
            Identity = reader.GetString(4),
            Version = reader.GetInt32(5),
            KeySlot = reader.GetInt32(6),
            TimestampMinutes = reader.GetInt64(7),
            MessageHex = reader.GetString(8),
            SampleRate = reader.GetInt32(9),
            Channels = reader.GetInt32(10),
            SampleCount = reader.GetInt64(11),
            PcmSha256 = reader.GetString(12),
            ChromaprintBlob = reader.IsDBNull(13) ? Array.Empty<byte>() : (byte[])reader[13],
            FingerprintLen = reader.GetInt32(14),
            FpConfigId = reader.GetInt32(15)
        };
    }
}
