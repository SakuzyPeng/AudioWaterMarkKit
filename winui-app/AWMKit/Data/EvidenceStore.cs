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

        var query = "SELECT id, file_path, file_hash, message, pattern, tag, created_at FROM audio_evidence ORDER BY created_at DESC";

        using var cmd = _database.Connection.CreateCommand();
        cmd.CommandText = query;

        using var reader = await cmd.ExecuteReaderAsync();
        while (await reader.ReadAsync())
        {
            records.Add(new EvidenceRecord
            {
                Id = reader.GetInt32(0),
                FilePath = reader.GetString(1),
                FileHash = reader.GetString(2),
                Message = reader.GetString(3),
                Pattern = reader.GetString(4),
                Tag = reader.GetString(5),
                CreatedAt = DateTime.Parse(reader.GetString(6))
            });
        }

        return records;
    }

    /// <summary>
    /// Gets an evidence record by file hash.
    /// </summary>
    public async Task<EvidenceRecord?> GetByHashAsync(string fileHash)
    {
        if (_database.Connection is null)
        {
            return null;
        }

        var query = "SELECT id, file_path, file_hash, message, pattern, tag, created_at FROM audio_evidence WHERE file_hash = @hash";

        using var cmd = _database.Connection.CreateCommand();
        cmd.CommandText = query;
        cmd.Parameters.AddWithValue("@hash", fileHash);

        using var reader = await cmd.ExecuteReaderAsync();
        if (await reader.ReadAsync())
        {
            return new EvidenceRecord
            {
                Id = reader.GetInt32(0),
                FilePath = reader.GetString(1),
                FileHash = reader.GetString(2),
                Message = reader.GetString(3),
                Pattern = reader.GetString(4),
                Tag = reader.GetString(5),
                CreatedAt = DateTime.Parse(reader.GetString(6))
            };
        }

        return null;
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

        var query = "SELECT id, file_path, file_hash, message, pattern, tag, created_at FROM audio_evidence WHERE tag = @tag ORDER BY created_at DESC";

        using var cmd = _database.Connection.CreateCommand();
        cmd.CommandText = query;
        cmd.Parameters.AddWithValue("@tag", tag);

        using var reader = await cmd.ExecuteReaderAsync();
        while (await reader.ReadAsync())
        {
            records.Add(new EvidenceRecord
            {
                Id = reader.GetInt32(0),
                FilePath = reader.GetString(1),
                FileHash = reader.GetString(2),
                Message = reader.GetString(3),
                Pattern = reader.GetString(4),
                Tag = reader.GetString(5),
                CreatedAt = DateTime.Parse(reader.GetString(6))
            });
        }

        return records;
    }

    /// <summary>
    /// Saves a new evidence record.
    /// </summary>
    public async Task<bool> SaveAsync(string filePath, string fileHash, string message, string pattern, string tag)
    {
        if (_database.Connection is null)
        {
            return false;
        }

        var now = DateTime.UtcNow.ToString("o");

        var insert = @"
INSERT INTO audio_evidence (file_path, file_hash, message, pattern, tag, created_at)
VALUES (@filePath, @fileHash, @message, @pattern, @tag, @now)
ON CONFLICT(file_hash) DO UPDATE SET
    file_path = @filePath,
    message = @message,
    pattern = @pattern,
    tag = @tag";

        try
        {
            using var cmd = _database.Connection.CreateCommand();
            cmd.CommandText = insert;
            cmd.Parameters.AddWithValue("@filePath", filePath);
            cmd.Parameters.AddWithValue("@fileHash", fileHash);
            cmd.Parameters.AddWithValue("@message", message);
            cmd.Parameters.AddWithValue("@pattern", pattern);
            cmd.Parameters.AddWithValue("@tag", tag);
            cmd.Parameters.AddWithValue("@now", now);

            await cmd.ExecuteNonQueryAsync();
            return true;
        }
        catch (SqliteException)
        {
            return false;
        }
    }

    /// <summary>
    /// Deletes an evidence record by file hash.
    /// </summary>
    public async Task<bool> DeleteByHashAsync(string fileHash)
    {
        if (_database.Connection is null)
        {
            return false;
        }

        var delete = "DELETE FROM audio_evidence WHERE file_hash = @hash";

        using var cmd = _database.Connection.CreateCommand();
        cmd.CommandText = delete;
        cmd.Parameters.AddWithValue("@hash", fileHash);

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

        var delete = "DELETE FROM audio_evidence WHERE tag = @tag";

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

        var query = "SELECT COUNT(*) FROM audio_evidence";

        using var cmd = _database.Connection.CreateCommand();
        cmd.CommandText = query;

        var result = await cmd.ExecuteScalarAsync();
        return result is long count ? (int)count : 0;
    }
}
