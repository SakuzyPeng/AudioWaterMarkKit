using AWMKit.Models;
using AWMKit.Native;
using System;
using System.Collections.Generic;
using System.Linq;
using System.Threading.Tasks;

namespace AWMKit.Data;

/// <summary>
/// Repository for tag_mappings via Rust FFI database APIs.
/// </summary>
public sealed class TagMappingStore
{
    public TagMappingStore(AppDatabase database)
    {
        _ = database;
    }

    public async Task<List<TagMapping>> ListAllAsync()
    {
        return await ListRecentAsync(int.MaxValue);
    }

    public async Task<List<TagMapping>> ListRecentAsync(int limit = 200)
    {
        await Task.CompletedTask;
        var (rows, error) = AwmDatabaseBridge.ListTagMappings(limit);
        return error == AwmError.Ok ? rows : [];
    }

    public async Task<List<TagMapping>> ListFilteredAsync(string? search, int limit = 200)
    {
        var all = await ListRecentAsync(limit);
        if (string.IsNullOrWhiteSpace(search))
        {
            return all;
        }

        var query = search.Trim();
        return all
            .Where(mapping =>
                mapping.Username.Contains(query, StringComparison.OrdinalIgnoreCase) ||
                mapping.Tag.Contains(query, StringComparison.OrdinalIgnoreCase))
            .ToList();
    }

    public async Task<TagMapping?> GetByIdentityAsync(string identity)
    {
        await Task.CompletedTask;
        var (tag, error) = AwmDatabaseBridge.LookupTag(identity);
        if (error != AwmError.Ok || string.IsNullOrWhiteSpace(tag))
        {
            return null;
        }

        return new TagMapping
        {
            Username = identity.Trim(),
            Tag = tag,
            CreatedAtUnix = DateTimeOffset.UtcNow.ToUnixTimeSeconds()
        };
    }

    public async Task<TagMapping?> GetByTagAsync(string tag)
    {
        var all = await ListRecentAsync(5000);
        return all.FirstOrDefault(item => string.Equals(item.Tag, tag, StringComparison.OrdinalIgnoreCase));
    }

    public async Task<bool> SaveAsync(string identity, string tag, string? displayName = null)
    {
        await Task.CompletedTask;
        _ = displayName;

        var normalizedIdentity = identity.Trim();
        var normalizedTag = tag.Trim().ToUpperInvariant();
        if (string.IsNullOrWhiteSpace(normalizedIdentity) || string.IsNullOrWhiteSpace(normalizedTag))
        {
            return false;
        }

        var existing = await GetByIdentityAsync(normalizedIdentity);
        if (existing is not null)
        {
            return string.Equals(existing.Tag, normalizedTag, StringComparison.OrdinalIgnoreCase);
        }

        var (inserted, error) = AwmDatabaseBridge.SaveTagIfAbsent(normalizedIdentity, normalizedTag);
        return error == AwmError.Ok && inserted;
    }

    public async Task<bool> SaveIfAbsentAsync(string username, string tag)
    {
        await Task.CompletedTask;
        var (inserted, error) = AwmDatabaseBridge.SaveTagIfAbsent(username.Trim(), tag.Trim().ToUpperInvariant());
        return error == AwmError.Ok && inserted;
    }

    public async Task<bool> DeleteByIdentityAsync(string identity)
    {
        await Task.CompletedTask;
        var (deleted, error) = AwmDatabaseBridge.RemoveTagMappings([identity]);
        return error == AwmError.Ok && deleted > 0;
    }

    public async Task<bool> DeleteByTagAsync(string tag)
    {
        var mapping = await GetByTagAsync(tag);
        if (mapping is null)
        {
            return false;
        }
        return await DeleteByIdentityAsync(mapping.Username);
    }

    public async Task<int> RemoveByUsernamesAsync(IEnumerable<string> usernames)
    {
        await Task.CompletedTask;
        var (deleted, error) = AwmDatabaseBridge.RemoveTagMappings(usernames);
        return error == AwmError.Ok ? deleted : 0;
    }

    public async Task<int> CountAsync()
    {
        await Task.CompletedTask;
        var (tagCount, _, error) = AwmDatabaseBridge.GetSummary();
        return error == AwmError.Ok ? (int)Math.Min(tagCount, int.MaxValue) : 0;
    }
}
