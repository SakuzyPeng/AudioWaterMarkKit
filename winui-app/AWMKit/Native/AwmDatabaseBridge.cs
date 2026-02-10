using AWMKit.Models;
using System;
using System.Collections.Generic;
using System.Linq;
using System.Runtime.InteropServices;
using System.Text.Json;
using System.Text.Json.Serialization;

namespace AWMKit.Native;

/// <summary>
/// High-level database bridge backed by Rust FFI.
/// </summary>
public static class AwmDatabaseBridge
{
    private static readonly JsonSerializerOptions JsonOptions = new(JsonSerializerDefaults.Web);

    public static (ulong tagCount, ulong evidenceCount, AwmError error) GetSummary()
    {
        var tagPtr = Marshal.AllocHGlobal(sizeof(ulong));
        var evidencePtr = Marshal.AllocHGlobal(sizeof(ulong));
        try
        {
            Marshal.WriteInt64(tagPtr, 0);
            Marshal.WriteInt64(evidencePtr, 0);

            int code = AwmNative.awm_db_summary(tagPtr, evidencePtr);
            var error = (AwmError)code;
            if (error != AwmError.Ok)
            {
                return (0, 0, error);
            }

            ulong tagCount = (ulong)Marshal.ReadInt64(tagPtr);
            ulong evidenceCount = (ulong)Marshal.ReadInt64(evidencePtr);
            return (tagCount, evidenceCount, AwmError.Ok);
        }
        finally
        {
            Marshal.FreeHGlobal(tagPtr);
            Marshal.FreeHGlobal(evidencePtr);
        }
    }

    public static (List<TagMapping> rows, AwmError error) ListTagMappings(int limit = 200)
    {
        uint normalized = NormalizeLimit(limit);
        var (json, error) = ReadJsonString((outBuf, outLen, outRequiredLen) =>
            AwmNative.awm_db_tag_list_json(normalized, outBuf, outLen, outRequiredLen));
        if (error != AwmError.Ok || json is null)
        {
            return ([], error);
        }

        try
        {
            var rows = JsonSerializer.Deserialize<List<TagMappingRow>>(json, JsonOptions) ?? [];
            var mappings = rows.Select(row => new TagMapping
            {
                Username = row.Username,
                Tag = row.Tag,
                CreatedAtUnix = ClampToInt64(row.CreatedAt)
            }).ToList();
            return (mappings, AwmError.Ok);
        }
        catch
        {
            return ([], AwmError.AudiowmarkExec);
        }
    }

    public static (string? tag, AwmError error) LookupTag(string username)
    {
        var normalized = username?.Trim() ?? string.Empty;
        if (string.IsNullOrWhiteSpace(normalized))
        {
            return (null, AwmError.InvalidTag);
        }

        var (value, error) = ReadJsonString((outBuf, outLen, outRequiredLen) =>
            AwmNative.awm_db_tag_lookup(normalized, outBuf, outLen, outRequiredLen));
        if (error != AwmError.Ok)
        {
            return (null, error);
        }

        if (string.IsNullOrWhiteSpace(value))
        {
            return (null, AwmError.Ok);
        }
        return (value, AwmError.Ok);
    }

    public static (bool inserted, AwmError error) SaveTagIfAbsent(string username, string tag)
    {
        var insertedPtr = Marshal.AllocHGlobal(1);
        try
        {
            Marshal.WriteByte(insertedPtr, 0);
            int code = AwmNative.awm_db_tag_save_if_absent(username, tag, insertedPtr);
            var error = (AwmError)code;
            if (error != AwmError.Ok)
            {
                return (false, error);
            }

            return (Marshal.ReadByte(insertedPtr) != 0, AwmError.Ok);
        }
        finally
        {
            Marshal.FreeHGlobal(insertedPtr);
        }
    }

    public static (int deleted, AwmError error) RemoveTagMappings(IEnumerable<string> usernames)
    {
        var normalized = usernames
            .Where(value => !string.IsNullOrWhiteSpace(value))
            .Select(value => value.Trim())
            .Distinct(StringComparer.OrdinalIgnoreCase)
            .ToArray();
        var payload = JsonSerializer.Serialize(normalized, JsonOptions);

        return ExecuteDeleteJson(payload, AwmNative.awm_db_tag_remove_json);
    }

    public static (List<EvidenceRecord> rows, AwmError error) ListEvidence(int limit = 200)
    {
        uint normalized = NormalizeLimit(limit);
        var (json, error) = ReadJsonString((outBuf, outLen, outRequiredLen) =>
            AwmNative.awm_db_evidence_list_json(normalized, outBuf, outLen, outRequiredLen));
        if (error != AwmError.Ok || json is null)
        {
            return ([], error);
        }

        try
        {
            var rows = JsonSerializer.Deserialize<List<EvidenceRow>>(json, JsonOptions) ?? [];
            var evidence = rows.Select(row => new EvidenceRecord
            {
                Id = row.Id,
                CreatedAt = ClampToInt64(row.CreatedAt),
                FilePath = row.FilePath,
                Tag = row.Tag,
                Identity = row.Identity,
                Version = row.Version,
                KeySlot = row.KeySlot,
                TimestampMinutes = row.TimestampMinutes,
                MessageHex = row.MessageHex,
                SampleRate = row.SampleRate,
                Channels = row.Channels,
                SampleCount = row.SampleCount,
                PcmSha256 = row.PcmSha256,
                KeyId = row.KeyId,
                ChromaprintBlob = DecodeHex(row.ChromaprintBlob),
                FingerprintLen = row.FingerprintLen,
                FpConfigId = row.FpConfigId
            }).ToList();
            return (evidence, AwmError.Ok);
        }
        catch
        {
            return ([], AwmError.AudiowmarkExec);
        }
    }

    public static (int deleted, AwmError error) RemoveEvidence(IEnumerable<long> ids)
    {
        var payload = JsonSerializer.Serialize(ids.Distinct().ToArray(), JsonOptions);
        return ExecuteDeleteJson(payload, AwmNative.awm_db_evidence_remove_json);
    }

    private static (int deleted, AwmError error) ExecuteDeleteJson(
        string payload,
        Func<string, IntPtr, int> invoker)
    {
        var deletedPtr = Marshal.AllocHGlobal(sizeof(uint));
        try
        {
            Marshal.WriteInt32(deletedPtr, 0);
            int code = invoker(payload, deletedPtr);
            var error = (AwmError)code;
            if (error != AwmError.Ok)
            {
                return (0, error);
            }

            return (Marshal.ReadInt32(deletedPtr), AwmError.Ok);
        }
        finally
        {
            Marshal.FreeHGlobal(deletedPtr);
        }
    }

    private static (string? value, AwmError error) ReadJsonString(
        Func<IntPtr, nuint, IntPtr, int> invoker)
    {
        var requiredPtr = Marshal.AllocHGlobal(IntPtr.Size);
        try
        {
            Marshal.WriteInt64(requiredPtr, 0);
            int first = invoker(IntPtr.Zero, 0, requiredPtr);
            var firstError = (AwmError)first;
            if (firstError != AwmError.Ok)
            {
                return (null, firstError);
            }

            nuint required = ReadNuint(requiredPtr);
            if (required == 0)
            {
                return (string.Empty, AwmError.Ok);
            }

            var buffer = Marshal.AllocHGlobal((int)required);
            try
            {
                int second = invoker(buffer, required, requiredPtr);
                var secondError = (AwmError)second;
                if (secondError != AwmError.Ok)
                {
                    return (null, secondError);
                }
                return (Marshal.PtrToStringUTF8(buffer), AwmError.Ok);
            }
            finally
            {
                Marshal.FreeHGlobal(buffer);
            }
        }
        finally
        {
            Marshal.FreeHGlobal(requiredPtr);
        }
    }

    private static nuint ReadNuint(IntPtr pointer)
    {
        return IntPtr.Size == 8
            ? (nuint)Marshal.ReadInt64(pointer)
            : (nuint)Marshal.ReadInt32(pointer);
    }

    private static uint NormalizeLimit(int limit)
    {
        if (limit <= 0)
        {
            return 200;
        }
        if (limit >= int.MaxValue)
        {
            return uint.MaxValue;
        }
        return (uint)limit;
    }

    private static long ClampToInt64(ulong value)
    {
        return value > long.MaxValue ? long.MaxValue : (long)value;
    }

    private static long ClampToInt64(long value) => value;

    private static byte[] DecodeHex(string? hex)
    {
        if (string.IsNullOrWhiteSpace(hex))
        {
            return Array.Empty<byte>();
        }

        try
        {
            return Convert.FromHexString(hex);
        }
        catch
        {
            return Array.Empty<byte>();
        }
    }

    private sealed class TagMappingRow
    {
        [JsonPropertyName("username")]
        public string Username { get; set; } = string.Empty;

        [JsonPropertyName("tag")]
        public string Tag { get; set; } = string.Empty;

        [JsonPropertyName("created_at")]
        public ulong CreatedAt { get; set; }
    }

    private sealed class EvidenceRow
    {
        [JsonPropertyName("id")]
        public long Id { get; set; }

        [JsonPropertyName("created_at")]
        public ulong CreatedAt { get; set; }

        [JsonPropertyName("file_path")]
        public string FilePath { get; set; } = string.Empty;

        [JsonPropertyName("tag")]
        public string Tag { get; set; } = string.Empty;

        [JsonPropertyName("identity")]
        public string Identity { get; set; } = string.Empty;

        [JsonPropertyName("version")]
        public int Version { get; set; }

        [JsonPropertyName("key_slot")]
        public int KeySlot { get; set; }

        [JsonPropertyName("timestamp_minutes")]
        public long TimestampMinutes { get; set; }

        [JsonPropertyName("message_hex")]
        public string MessageHex { get; set; } = string.Empty;

        [JsonPropertyName("sample_rate")]
        public int SampleRate { get; set; }

        [JsonPropertyName("channels")]
        public int Channels { get; set; }

        [JsonPropertyName("sample_count")]
        public long SampleCount { get; set; }

        [JsonPropertyName("pcm_sha256")]
        public string PcmSha256 { get; set; } = string.Empty;

        [JsonPropertyName("key_id")]
        public string? KeyId { get; set; }

        [JsonPropertyName("chromaprint_blob")]
        public string ChromaprintBlob { get; set; } = string.Empty;

        [JsonPropertyName("fingerprint_len")]
        public int FingerprintLen { get; set; }

        [JsonPropertyName("fp_config_id")]
        public int FpConfigId { get; set; }
    }
}
