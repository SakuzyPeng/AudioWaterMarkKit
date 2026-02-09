using AWMKit.Models;
using Microsoft.Data.Sqlite;
using System;
using System.Collections.Generic;
using System.Threading.Tasks;

namespace AWMKit.Data;

/// <summary>
/// Repository for tag_mappings table operations.
/// </summary>
public sealed class TagMappingStore
{
    private readonly AppDatabase _database;

    public TagMappingStore(AppDatabase database)
    {
        _database = database;
    }

    /// <summary>
    /// Lists all tag mappings in username order.
    /// </summary>
    public async Task<List<TagMapping>> ListAllAsync()
    {
        var mappings = new List<TagMapping>();

        if (_database.Connection is null)
        {
            return mappings;
        }

        const string query = """
            SELECT username, tag, created_at
            FROM tag_mappings
            ORDER BY username COLLATE NOCASE ASC
            """;

        using var cmd = _database.Connection.CreateCommand();
        cmd.CommandText = query;

        using var reader = await cmd.ExecuteReaderAsync();
        while (await reader.ReadAsync())
        {
            mappings.Add(new TagMapping
            {
                Username = reader.GetString(0),
                Tag = reader.GetString(1),
                CreatedAtUnix = reader.GetInt64(2)
            });
        }

        return mappings;
    }

    /// <summary>
    /// Gets a tag mapping by username (legacy API name kept for compatibility).
    /// </summary>
    public async Task<TagMapping?> GetByIdentityAsync(string identity)
    {
        if (_database.Connection is null)
        {
            return null;
        }

        const string query = """
            SELECT username, tag, created_at
            FROM tag_mappings
            WHERE username = @username COLLATE NOCASE
            LIMIT 1
            """;

        using var cmd = _database.Connection.CreateCommand();
        cmd.CommandText = query;
        cmd.Parameters.AddWithValue("@username", identity);

        using var reader = await cmd.ExecuteReaderAsync();
        if (!await reader.ReadAsync())
        {
            return null;
        }

        return new TagMapping
        {
            Username = reader.GetString(0),
            Tag = reader.GetString(1),
            CreatedAtUnix = reader.GetInt64(2)
        };
    }

    /// <summary>
    /// Gets a tag mapping by tag.
    /// </summary>
    public async Task<TagMapping?> GetByTagAsync(string tag)
    {
        if (_database.Connection is null)
        {
            return null;
        }

        const string query = """
            SELECT username, tag, created_at
            FROM tag_mappings
            WHERE tag = @tag
            LIMIT 1
            """;

        using var cmd = _database.Connection.CreateCommand();
        cmd.CommandText = query;
        cmd.Parameters.AddWithValue("@tag", tag);

        using var reader = await cmd.ExecuteReaderAsync();
        if (!await reader.ReadAsync())
        {
            return null;
        }

        return new TagMapping
        {
            Username = reader.GetString(0),
            Tag = reader.GetString(1),
            CreatedAtUnix = reader.GetInt64(2)
        };
    }

    /// <summary>
    /// Saves or updates a tag mapping.
    /// displayName is ignored because Rust schema has no such field.
    /// </summary>
    public async Task<bool> SaveAsync(string identity, string tag, string? displayName = null)
    {
        if (_database.Connection is null)
        {
            return false;
        }

        _ = displayName;
        var now = DateTimeOffset.UtcNow.ToUnixTimeSeconds();

        const string upsert = """
            INSERT INTO tag_mappings (username, tag, created_at)
            VALUES (@username, @tag, @createdAt)
            ON CONFLICT(username) DO UPDATE SET
                tag = excluded.tag,
                created_at = excluded.created_at
            """;

        try
        {
            using var cmd = _database.Connection.CreateCommand();
            cmd.CommandText = upsert;
            cmd.Parameters.AddWithValue("@username", identity);
            cmd.Parameters.AddWithValue("@tag", tag);
            cmd.Parameters.AddWithValue("@createdAt", now);
            await cmd.ExecuteNonQueryAsync();
            return true;
        }
        catch (SqliteException)
        {
            return false;
        }
    }

    /// <summary>
    /// Saves a mapping only when username does not exist yet.
    /// Returns true when inserted, false when already exists or on failure.
    /// </summary>
    public async Task<bool> SaveIfAbsentAsync(string username, string tag)
    {
        var existing = await GetByIdentityAsync(username);
        if (existing is not null)
        {
            return false;
        }

        return await SaveAsync(username, tag);
    }

    /// <summary>
    /// Deletes a mapping by username (legacy API name kept for compatibility).
    /// </summary>
    public async Task<bool> DeleteByIdentityAsync(string identity)
    {
        if (_database.Connection is null)
        {
            return false;
        }

        const string delete = """
            DELETE FROM tag_mappings
            WHERE username = @username COLLATE NOCASE
            """;

        using var cmd = _database.Connection.CreateCommand();
        cmd.CommandText = delete;
        cmd.Parameters.AddWithValue("@username", identity);

        var rowsAffected = await cmd.ExecuteNonQueryAsync();
        return rowsAffected > 0;
    }

    /// <summary>
    /// Deletes a mapping by tag.
    /// </summary>
    public async Task<bool> DeleteByTagAsync(string tag)
    {
        if (_database.Connection is null)
        {
            return false;
        }

        const string delete = "DELETE FROM tag_mappings WHERE tag = @tag";

        using var cmd = _database.Connection.CreateCommand();
        cmd.CommandText = delete;
        cmd.Parameters.AddWithValue("@tag", tag);

        var rowsAffected = await cmd.ExecuteNonQueryAsync();
        return rowsAffected > 0;
    }
}
