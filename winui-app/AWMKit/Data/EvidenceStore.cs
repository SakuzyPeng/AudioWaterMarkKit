using AWMKit.Models;
using AWMKit.Native;
using System;
using System.Collections.Generic;
using System.Linq;
using System.Threading.Tasks;

namespace AWMKit.Data;

/// <summary>
/// Repository for audio_evidence via Rust FFI database APIs.
/// </summary>
public sealed class EvidenceStore
{
    public EvidenceStore(AppDatabase database)
    {
        _ = database;
    }

    public async Task<List<EvidenceRecord>> ListAllAsync()
    {
        return await ListRecentAsync(int.MaxValue);
    }

    public async Task<List<EvidenceRecord>> ListRecentAsync(int limit = 200)
    {
        await Task.CompletedTask;
        var (rows, error) = AwmDatabaseBridge.ListEvidence(limit);
        return error == AwmError.Ok ? rows : [];
    }

    public async Task<List<EvidenceRecord>> ListFilteredAsync(
        string? identity = null,
        string? tag = null,
        int? keySlot = null,
        int limit = 200)
    {
        var rows = await ListRecentAsync(limit);
        if (!string.IsNullOrWhiteSpace(identity))
        {
            rows = rows.Where(row => row.Identity.Equals(identity.Trim(), StringComparison.OrdinalIgnoreCase)).ToList();
        }
        if (!string.IsNullOrWhiteSpace(tag))
        {
            rows = rows.Where(row => row.Tag.Equals(tag.Trim(), StringComparison.OrdinalIgnoreCase)).ToList();
        }
        if (keySlot.HasValue)
        {
            rows = rows.Where(row => row.KeySlot == keySlot.Value).ToList();
        }
        return rows;
    }

    public async Task<EvidenceRecord?> GetByHashAsync(string fileHash)
    {
        var rows = await ListRecentAsync(5000);
        return rows.FirstOrDefault(row => row.PcmSha256.Equals(fileHash, StringComparison.OrdinalIgnoreCase));
    }

    public async Task<List<EvidenceRecord>> ListByTagAsync(string tag)
    {
        var rows = await ListRecentAsync(5000);
        return rows.Where(row => row.Tag.Equals(tag, StringComparison.OrdinalIgnoreCase)).ToList();
    }

    public async Task<bool> SaveAsync(string filePath, string fileHash, string message, string pattern, string tag)
    {
        await Task.CompletedTask;
        _ = (filePath, fileHash, message, pattern, tag);
        return false;
    }

    public async Task<bool> DeleteByHashAsync(string fileHash)
    {
        var row = await GetByHashAsync(fileHash);
        if (row is null)
        {
            return false;
        }
        var deleted = await RemoveByIdsAsync([row.Id]);
        return deleted > 0;
    }

    public async Task<int> DeleteByTagAsync(string tag)
    {
        var rows = await ListByTagAsync(tag);
        return await RemoveByIdsAsync(rows.Select(row => row.Id));
    }

    public async Task<int> CountAsync()
    {
        await Task.CompletedTask;
        var (_, evidenceCount, error) = AwmDatabaseBridge.GetSummary();
        return error == AwmError.Ok ? (int)Math.Min(evidenceCount, int.MaxValue) : 0;
    }

    public async Task<int> CountAllAsync()
    {
        return await CountAsync();
    }

    public async Task<int> RemoveByIdsAsync(IEnumerable<long> ids)
    {
        await Task.CompletedTask;
        var (deleted, error) = AwmDatabaseBridge.RemoveEvidence(ids);
        return error == AwmError.Ok ? deleted : 0;
    }
}
