using System;
using System.Threading.Tasks;

namespace AWMKit.Data;

/// <summary>
/// Repository for app_settings table operations.
/// </summary>
public sealed class AppSettingsStore
{
    private const string ActiveKeySlotKey = "active_key_slot";
    private readonly AppDatabase _database;

    public AppSettingsStore(AppDatabase database)
    {
        _database = database;
    }

    public async Task<int> GetActiveKeySlotAsync()
    {
        if (_database.Connection is null)
        {
            return 0;
        }

        const string query = """
            SELECT value
            FROM app_settings
            WHERE key = @key
            LIMIT 1
            """;

        using var cmd = _database.Connection.CreateCommand();
        cmd.CommandText = query;
        cmd.Parameters.AddWithValue("@key", ActiveKeySlotKey);

        var result = await cmd.ExecuteScalarAsync();
        if (result is string value && int.TryParse(value, out var parsed))
        {
            return Math.Clamp(parsed, 0, 31);
        }

        return 0;
    }

    public async Task SaveActiveKeySlotAsync(int slot)
    {
        if (_database.Connection is null)
        {
            return;
        }

        var normalized = Math.Clamp(slot, 0, 31);
        var now = DateTimeOffset.UtcNow.ToUnixTimeSeconds();

        const string upsert = """
            INSERT INTO app_settings (key, value, updated_at)
            VALUES (@key, @value, @updatedAt)
            ON CONFLICT(key) DO UPDATE SET
                value = excluded.value,
                updated_at = excluded.updated_at
            """;

        using var cmd = _database.Connection.CreateCommand();
        cmd.CommandText = upsert;
        cmd.Parameters.AddWithValue("@key", ActiveKeySlotKey);
        cmd.Parameters.AddWithValue("@value", normalized.ToString());
        cmd.Parameters.AddWithValue("@updatedAt", now);
        await cmd.ExecuteNonQueryAsync();
    }
}
