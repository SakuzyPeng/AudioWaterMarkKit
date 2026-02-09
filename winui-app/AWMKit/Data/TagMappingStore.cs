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
    /// Lists all tag mappings.
    /// </summary>
    public async Task<List<TagMapping>> ListAllAsync()
    {
        var mappings = new List<TagMapping>();

        if (_database.Connection is null)
        {
            return mappings;
        }

        var query = "SELECT identity, tag, display_name, created_at, updated_at FROM tag_mappings ORDER BY updated_at DESC";

        using var cmd = _database.Connection.CreateCommand();
        cmd.CommandText = query;

        using var reader = await cmd.ExecuteReaderAsync();
        while (await reader.ReadAsync())
        {
            mappings.Add(new TagMapping
            {
                Identity = reader.GetString(0),
                Tag = reader.GetString(1),
                DisplayName = reader.IsDBNull(2) ? null : reader.GetString(2),
                CreatedAt = DateTime.Parse(reader.GetString(3)),
                UpdatedAt = DateTime.Parse(reader.GetString(4))
            });
        }

        return mappings;
    }

    /// <summary>
    /// Gets a tag mapping by identity.
    /// </summary>
    public async Task<TagMapping?> GetByIdentityAsync(string identity)
    {
        if (_database.Connection is null)
        {
            return null;
        }

        var query = "SELECT identity, tag, display_name, created_at, updated_at FROM tag_mappings WHERE identity = @identity";

        using var cmd = _database.Connection.CreateCommand();
        cmd.CommandText = query;
        cmd.Parameters.AddWithValue("@identity", identity);

        using var reader = await cmd.ExecuteReaderAsync();
        if (await reader.ReadAsync())
        {
            return new TagMapping
            {
                Identity = reader.GetString(0),
                Tag = reader.GetString(1),
                DisplayName = reader.IsDBNull(2) ? null : reader.GetString(2),
                CreatedAt = DateTime.Parse(reader.GetString(3)),
                UpdatedAt = DateTime.Parse(reader.GetString(4))
            };
        }

        return null;
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

        var query = "SELECT identity, tag, display_name, created_at, updated_at FROM tag_mappings WHERE tag = @tag";

        using var cmd = _database.Connection.CreateCommand();
        cmd.CommandText = query;
        cmd.Parameters.AddWithValue("@tag", tag);

        using var reader = await cmd.ExecuteReaderAsync();
        if (await reader.ReadAsync())
        {
            return new TagMapping
            {
                Identity = reader.GetString(0),
                Tag = reader.GetString(1),
                DisplayName = reader.IsDBNull(2) ? null : reader.GetString(2),
                CreatedAt = DateTime.Parse(reader.GetString(3)),
                UpdatedAt = DateTime.Parse(reader.GetString(4))
            };
        }

        return null;
    }

    /// <summary>
    /// Saves or updates a tag mapping.
    /// </summary>
    public async Task<bool> SaveAsync(string identity, string tag, string? displayName = null)
    {
        if (_database.Connection is null)
        {
            return false;
        }

        var now = DateTime.UtcNow.ToString("o");

        var upsert = @"
INSERT INTO tag_mappings (identity, tag, display_name, created_at, updated_at)
VALUES (@identity, @tag, @displayName, @now, @now)
ON CONFLICT(identity) DO UPDATE SET
    tag = @tag,
    display_name = @displayName,
    updated_at = @now";

        try
        {
            using var cmd = _database.Connection.CreateCommand();
            cmd.CommandText = upsert;
            cmd.Parameters.AddWithValue("@identity", identity);
            cmd.Parameters.AddWithValue("@tag", tag);
            cmd.Parameters.AddWithValue("@displayName", displayName ?? (object)DBNull.Value);
            cmd.Parameters.AddWithValue("@now", now);

            await cmd.ExecuteNonQueryAsync();
            return true;
        }
        catch (SqliteException)
        {
            // Constraint violation (duplicate tag)
            return false;
        }
    }

    /// <summary>
    /// Deletes a tag mapping by identity.
    /// </summary>
    public async Task<bool> DeleteByIdentityAsync(string identity)
    {
        if (_database.Connection is null)
        {
            return false;
        }

        var delete = "DELETE FROM tag_mappings WHERE identity = @identity";

        using var cmd = _database.Connection.CreateCommand();
        cmd.CommandText = delete;
        cmd.Parameters.AddWithValue("@identity", identity);

        var rowsAffected = await cmd.ExecuteNonQueryAsync();
        return rowsAffected > 0;
    }

    /// <summary>
    /// Deletes a tag mapping by tag.
    /// </summary>
    public async Task<bool> DeleteByTagAsync(string tag)
    {
        if (_database.Connection is null)
        {
            return false;
        }

        var delete = "DELETE FROM tag_mappings WHERE tag = @tag";

        using var cmd = _database.Connection.CreateCommand();
        cmd.CommandText = delete;
        cmd.Parameters.AddWithValue("@tag", tag);

        var rowsAffected = await cmd.ExecuteNonQueryAsync();
        return rowsAffected > 0;
    }
}
