using AWMKit.Data;
using Microsoft.Data.Sqlite;
using Xunit;

[assembly: CollectionBehavior(DisableTestParallelization = true)]

namespace AWMKit.Data.Tests;

public sealed class DatabaseStoreTests
{
    [Fact]
    public async Task DatabaseSchema_MatchesRustCoreTables()
    {
        using var db = new AppDatabase();
        Assert.True(await db.OpenAsync());

        await using var conn = new SqliteConnection($"Data Source={db.DatabasePath}");
        await conn.OpenAsync();

        var tagColumns = await QueryColumnsAsync(conn, "tag_mappings");
        Assert.Equal(["username", "tag", "created_at"], tagColumns);

        var evidenceColumns = await QueryColumnsAsync(conn, "audio_evidence");
        Assert.Equal(
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
                "key_id",
                "is_forced_embed",
                "snr_db",
                "snr_status",
                "chromaprint_blob",
                "fingerprint_len",
                "fp_config_id"
            ],
            evidenceColumns
        );
    }

    [Fact]
    public async Task TagMappings_Crud_WorksAgainstRustSchema()
    {
        using var db = new AppDatabase();
        Assert.True(await db.OpenAsync());

        var username = $"test-{Guid.NewGuid():N}".ToUpperInvariant();
        const string tag = "ABCDEFGH";
        var now = DateTimeOffset.UtcNow.ToUnixTimeSeconds();

        try
        {
            await using var conn = new SqliteConnection($"Data Source={db.DatabasePath}");
            await conn.OpenAsync();

            await using (var insert = conn.CreateCommand())
            {
                insert.CommandText = """
                    INSERT INTO tag_mappings (username, tag, created_at)
                    VALUES ($username, $tag, $created_at)
                    """;
                insert.Parameters.AddWithValue("$username", username);
                insert.Parameters.AddWithValue("$tag", tag);
                insert.Parameters.AddWithValue("$created_at", now);
                var affected = await insert.ExecuteNonQueryAsync();
                Assert.Equal(1, affected);
            }

            await using (var query = conn.CreateCommand())
            {
                query.CommandText = """
                    SELECT username, tag FROM tag_mappings
                    WHERE username = $username
                    """;
                query.Parameters.AddWithValue("$username", username);
                await using var reader = await query.ExecuteReaderAsync();
                Assert.True(await reader.ReadAsync());
                Assert.Equal(username, reader.GetString(0));
                Assert.Equal(tag, reader.GetString(1));
            }
        }
        finally
        {
            await using var conn = new SqliteConnection($"Data Source={db.DatabasePath}");
            await conn.OpenAsync();
            await using var delete = conn.CreateCommand();
            delete.CommandText = "DELETE FROM tag_mappings WHERE username = $username";
            delete.Parameters.AddWithValue("$username", username);
            _ = await delete.ExecuteNonQueryAsync();
        }
    }

    [Fact]
    public async Task AudioEvidence_Crud_WorksAgainstRustSchema()
    {
        using var db = new AppDatabase();
        Assert.True(await db.OpenAsync());

        var sha = $"sha256-{Guid.NewGuid():N}";
        var identity = $"ID-{Guid.NewGuid():N}"[..10];
        var createdAt = DateTimeOffset.UtcNow.ToUnixTimeSeconds();
        var timestampMinutes = createdAt / 60;

        try
        {
            await using var conn = new SqliteConnection($"Data Source={db.DatabasePath}");
            await conn.OpenAsync();

            await using (var insert = conn.CreateCommand())
            {
                insert.CommandText = """
                    INSERT INTO audio_evidence (
                        created_at, file_path, tag, identity, version, key_slot, timestamp_minutes,
                        message_hex, sample_rate, channels, sample_count, pcm_sha256, key_id, is_forced_embed,
                        snr_db, snr_status, chromaprint_blob, fingerprint_len, fp_config_id
                    ) VALUES (
                        $created_at, $file_path, $tag, $identity, $version, $key_slot, $timestamp_minutes,
                        $message_hex, $sample_rate, $channels, $sample_count, $pcm_sha256, $key_id, $is_forced_embed,
                        $snr_db, $snr_status, $chromaprint_blob, $fingerprint_len, $fp_config_id
                    )
                    """;
                insert.Parameters.AddWithValue("$created_at", createdAt);
                insert.Parameters.AddWithValue("$file_path", @"D:\test\out.wav");
                insert.Parameters.AddWithValue("$tag", "ABCDEFGH");
                insert.Parameters.AddWithValue("$identity", identity);
                insert.Parameters.AddWithValue("$version", 2);
                insert.Parameters.AddWithValue("$key_slot", 0);
                insert.Parameters.AddWithValue("$timestamp_minutes", timestampMinutes);
                insert.Parameters.AddWithValue("$message_hex", "00112233445566778899AABBCCDDEEFF");
                insert.Parameters.AddWithValue("$sample_rate", 48000);
                insert.Parameters.AddWithValue("$channels", 2);
                insert.Parameters.AddWithValue("$sample_count", 1000);
                insert.Parameters.AddWithValue("$pcm_sha256", sha);
                insert.Parameters.AddWithValue("$key_id", "TESTKEY001");
                insert.Parameters.AddWithValue("$is_forced_embed", 0);
                insert.Parameters.AddWithValue("$snr_db", DBNull.Value);
                insert.Parameters.AddWithValue("$snr_status", "unavailable");
                insert.Parameters.AddWithValue("$chromaprint_blob", Array.Empty<byte>());
                insert.Parameters.AddWithValue("$fingerprint_len", 0);
                insert.Parameters.AddWithValue("$fp_config_id", 2);
                var affected = await insert.ExecuteNonQueryAsync();
                Assert.Equal(1, affected);
            }

            await using (var query = conn.CreateCommand())
            {
                query.CommandText = """
                    SELECT pcm_sha256, tag, identity FROM audio_evidence
                    WHERE pcm_sha256 = $pcm_sha256
                    """;
                query.Parameters.AddWithValue("$pcm_sha256", sha);
                await using var reader = await query.ExecuteReaderAsync();
                Assert.True(await reader.ReadAsync());
                Assert.Equal(sha, reader.GetString(0));
                Assert.Equal("ABCDEFGH", reader.GetString(1));
                Assert.Equal(identity, reader.GetString(2));
            }
        }
        finally
        {
            await using var conn = new SqliteConnection($"Data Source={db.DatabasePath}");
            await conn.OpenAsync();
            await using var delete = conn.CreateCommand();
            delete.CommandText = "DELETE FROM audio_evidence WHERE pcm_sha256 = $pcm_sha256";
            delete.Parameters.AddWithValue("$pcm_sha256", sha);
            _ = await delete.ExecuteNonQueryAsync();
        }
    }

    private static async Task<List<string>> QueryColumnsAsync(SqliteConnection connection, string table)
    {
        var columns = new List<string>();
        await using var cmd = connection.CreateCommand();
        cmd.CommandText = $"PRAGMA table_info({table})";
        await using var reader = await cmd.ExecuteReaderAsync();
        while (await reader.ReadAsync())
        {
            columns.Add(reader.GetString(1));
        }

        return columns;
    }
}
