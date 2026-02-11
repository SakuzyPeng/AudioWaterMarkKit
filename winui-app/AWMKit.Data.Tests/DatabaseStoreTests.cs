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
    public async Task TagMappingStore_Crud_WorksAgainstRustSchema()
    {
        using var db = new AppDatabase();
        Assert.True(await db.OpenAsync());

        var username = $"test-{Guid.NewGuid():N}".ToUpperInvariant();
        const string tag = "ABCDEFGH";
        var store = new TagMappingStore(db);

        try
        {
            Assert.True(await store.SaveAsync(username, tag));
            var byTag = await store.GetByTagAsync(tag);
            Assert.NotNull(byTag);
            Assert.Equal(username, byTag!.Username);

            var byIdentity = await store.GetByIdentityAsync(username);
            Assert.NotNull(byIdentity);
            Assert.Equal(tag, byIdentity!.Tag);
        }
        finally
        {
            _ = await store.DeleteByIdentityAsync(username);
        }
    }

    [Fact]
    public async Task EvidenceStore_Crud_WorksAgainstRustSchema()
    {
        using var db = new AppDatabase();
        Assert.True(await db.OpenAsync());

        var sha = $"sha256-{Guid.NewGuid():N}";
        var store = new EvidenceStore(db);

        try
        {
            Assert.True(await store.SaveAsync("/tmp/out.wav", sha, "001122", "all", "ABCDEFGH"));
            var one = await store.GetByHashAsync(sha);
            Assert.NotNull(one);
            Assert.Equal(sha, one!.PcmSha256);
        }
        finally
        {
            _ = await store.DeleteByHashAsync(sha);
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
