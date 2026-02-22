using AWMKit.Models;
using System;
using System.Collections.Generic;
using System.Linq;
using System.Text;
using System.Runtime.InteropServices;
using System.Text.Json;
using System.Text.Json.Serialization;

namespace AWMKit.Native;

/// <summary>
/// Bridge for slot-aware key management operations via Rust KeyStore.
/// Backend order on Windows: keyring first, DPAPI file fallback.
/// All keys are 32 bytes (256-bit HMAC-SHA256 keys).
/// </summary>
public static class AwmKeyBridge
{
    private const int KeySize = 32;
    private const int LabelBufferSize = 512;
    private const int MinSlot = 0;
    private const int MaxSlot = 31;
    private static byte NormalizeSlot(int slot) => (byte)Math.Clamp(slot, MinSlot, MaxSlot);

    /// <summary>Checks if key exists in current active slot.</summary>
    public static bool KeyExists()
    {
        return AwmNative.awm_key_exists();
    }

    /// <summary>Checks if key exists in specific slot.</summary>
    public static bool KeyExistsInSlot(int slot)
    {
        return AwmNative.awm_key_exists_slot(NormalizeSlot(slot));
    }

    /// <summary>
    /// Gets the active key backend label from native layer.
    /// Examples: "keyring (service: ...)", "dpapi (...)", "none".
    /// </summary>
    public static (string? backend, AwmError error) GetBackendLabel()
    {
        var buffer = new byte[LabelBufferSize];
        var handle = GCHandle.Alloc(buffer, GCHandleType.Pinned);
        try
        {
            int code = AwmNative.awm_key_backend_label(handle.AddrOfPinnedObject(), (nuint)buffer.Length);
            var error = (AwmError)code;
            if (error != AwmError.Ok)
            {
                return (null, error);
            }

            var len = Array.IndexOf(buffer, (byte)0);
            if (len < 0) len = buffer.Length;
            var label = Encoding.UTF8.GetString(buffer, 0, len);
            return (label, AwmError.Ok);
        }
        finally
        {
            handle.Free();
        }
    }

    /// <summary>
    /// Loads key from current active slot.
    /// </summary>
    /// <returns>(key bytes, error code). Key is null if error occurred.</returns>
    public static (byte[]? key, AwmError error) LoadKey()
    {
        var keyBuffer = new byte[KeySize];
        var handle = GCHandle.Alloc(keyBuffer, GCHandleType.Pinned);
        try
        {
            int code = AwmNative.awm_key_load(handle.AddrOfPinnedObject(), (nuint)KeySize);
            if (code == 0)
            {
                return (keyBuffer, AwmError.Ok);
            }
            return (null, (AwmError)code);
        }
        finally
        {
            handle.Free();
        }
    }

    /// <summary>
    /// Loads key from specific slot.
    /// </summary>
    /// <returns>(key bytes, error code). Key is null if error occurred.</returns>
    public static (byte[]? key, AwmError error) LoadKeyInSlot(int slot)
    {
        var keyBuffer = new byte[KeySize];
        var handle = GCHandle.Alloc(keyBuffer, GCHandleType.Pinned);
        try
        {
            int code = AwmNative.awm_key_load_slot(NormalizeSlot(slot), handle.AddrOfPinnedObject(), (nuint)KeySize);
            if (code == 0)
            {
                return (keyBuffer, AwmError.Ok);
            }
            return (null, (AwmError)code);
        }
        finally
        {
            handle.Free();
        }
    }

    /// <summary>
    /// Saves key into specific slot. Overwrite is not allowed.
    /// </summary>
    public static AwmError SaveKeyInSlot(int slot, byte[] key)
    {
        if (key.Length != KeySize)
        {
            return AwmError.InvalidMessageLength;
        }

        var handle = GCHandle.Alloc(key, GCHandleType.Pinned);
        try
        {
            int code = AwmNative.awm_key_save_slot(NormalizeSlot(slot), handle.AddrOfPinnedObject(), (nuint)key.Length);
            return (AwmError)code;
        }
        finally
        {
            handle.Free();
        }
    }

    /// <summary>
    /// Gets current active slot.
    /// </summary>
    public static (int slot, AwmError error) GetActiveSlot()
    {
        var buffer = new byte[1];
        var handle = GCHandle.Alloc(buffer, GCHandleType.Pinned);
        try
        {
            int code = AwmNative.awm_key_active_slot_get(handle.AddrOfPinnedObject());
            var error = (AwmError)code;
            if (error != AwmError.Ok)
            {
                return (MinSlot, error);
            }

            return (buffer[0], AwmError.Ok);
        }
        finally
        {
            handle.Free();
        }
    }

    /// <summary>
    /// Sets current active slot.
    /// </summary>
    public static AwmError SetActiveSlot(int slot)
    {
        int code = AwmNative.awm_key_active_slot_set(NormalizeSlot(slot));
        return (AwmError)code;
    }

    /// <summary>
    /// Set human-readable label for specific slot.
    /// </summary>
    public static AwmError SetSlotLabel(int slot, string label)
    {
        int code = AwmNative.awm_key_slot_label_set(NormalizeSlot(slot), label);
        return (AwmError)code;
    }

    /// <summary>
    /// Clear human-readable label for specific slot.
    /// </summary>
    public static AwmError ClearSlotLabel(int slot)
    {
        int code = AwmNative.awm_key_slot_label_clear(NormalizeSlot(slot));
        return (AwmError)code;
    }

    /// <summary>
    /// Generates key and saves into current active slot.
    /// </summary>
    public static (byte[]? key, AwmError error) GenerateAndSaveKey()
    {
        var (slot, slotErr) = GetActiveSlot();
        if (slotErr != AwmError.Ok)
        {
            return (null, slotErr);
        }
        return GenerateAndSaveKeyInSlot(slot);
    }

    /// <summary>
    /// Generates key and saves into specific slot.
    /// </summary>
    public static (byte[]? key, AwmError error) GenerateAndSaveKeyInSlot(int slot)
    {
        var keyBuffer = new byte[KeySize];
        var handle = GCHandle.Alloc(keyBuffer, GCHandleType.Pinned);
        try
        {
            int code = AwmNative.awm_key_generate_and_save_slot(
                NormalizeSlot(slot),
                handle.AddrOfPinnedObject(),
                (nuint)KeySize);
            if (code == 0)
            {
                return (keyBuffer, AwmError.Ok);
            }
            return (null, (AwmError)code);
        }
        finally
        {
            handle.Free();
        }
    }

    /// <summary>
    /// Deletes key from current active slot and returns effective active slot after fallback.
    /// </summary>
    public static (int newActiveSlot, AwmError error) DeleteKey()
    {
        var (slot, slotErr) = GetActiveSlot();
        if (slotErr != AwmError.Ok)
        {
            return (MinSlot, slotErr);
        }
        return DeleteKeyInSlot(slot);
    }

    /// <summary>
    /// Deletes key from specific slot and returns effective active slot after fallback.
    /// </summary>
    public static (int newActiveSlot, AwmError error) DeleteKeyInSlot(int slot)
    {
        var slotBuffer = new byte[1];
        var handle = GCHandle.Alloc(slotBuffer, GCHandleType.Pinned);
        try
        {
            int code = AwmNative.awm_key_delete_slot(NormalizeSlot(slot), handle.AddrOfPinnedObject());
            var error = (AwmError)code;
            if (error != AwmError.Ok)
            {
                return (MinSlot, error);
            }
            return (slotBuffer[0], AwmError.Ok);
        }
        finally
        {
            handle.Free();
        }
    }

    /// <summary>
    /// Returns slot summaries for all 32 slots from Rust side.
    /// </summary>
    public static (List<KeySlotSummary> rows, AwmError error) GetSlotSummaries()
    {
        var (json, error) = ReadJsonString((outBuf, outLen, outRequiredLen) =>
            AwmNative.awm_key_slot_summaries_json(outBuf, outLen, outRequiredLen));
        if (error != AwmError.Ok || json is null)
        {
            return ([], error);
        }

        try
        {
            var payload = string.IsNullOrWhiteSpace(json) ? "[]" : json;
            var rows = JsonSerializer.Deserialize(
                payload,
                typeof(List<KeySlotSummaryRow>),
                AwmJsonContext.Default) as List<KeySlotSummaryRow> ?? [];
            var mapped = rows.Select(row => new KeySlotSummary
            {
                Slot = Math.Clamp(row.Slot, MinSlot, MaxSlot),
                IsActive = row.IsActive,
                HasKey = row.HasKey,
                KeyId = row.KeyId,
                Label = string.IsNullOrWhiteSpace(row.Label) ? null : row.Label,
                EvidenceCount = row.EvidenceCount,
                LastEvidenceAt = row.LastEvidenceAt,
                StatusText = row.StatusText ?? "empty",
                DuplicateOfSlots = row.DuplicateOfSlots?.Distinct().OrderBy(v => v).ToArray() ?? Array.Empty<int>()
            }).ToList();
            return (mapped, AwmError.Ok);
        }
        catch
        {
            return ([], AwmError.AudiowmarkExec);
        }
    }

    private static (string? value, AwmError error) ReadJsonString(
        Func<IntPtr, nuint, IntPtr, int> invoker)
    {
        var requiredPtr = Marshal.AllocHGlobal(IntPtr.Size);
        try
        {
            if (IntPtr.Size == 8)
            {
                Marshal.WriteInt64(requiredPtr, 0);
            }
            else
            {
                Marshal.WriteInt32(requiredPtr, 0);
            }

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

    internal sealed class KeySlotSummaryRow
    {
        [JsonPropertyName("slot")]
        public int Slot { get; set; }

        [JsonPropertyName("is_active")]
        public bool IsActive { get; set; }

        [JsonPropertyName("has_key")]
        public bool HasKey { get; set; }

        [JsonPropertyName("key_id")]
        public string? KeyId { get; set; }

        [JsonPropertyName("label")]
        public string? Label { get; set; }

        [JsonPropertyName("evidence_count")]
        public int EvidenceCount { get; set; }

        [JsonPropertyName("last_evidence_at")]
        public long? LastEvidenceAt { get; set; }

        [JsonPropertyName("status_text")]
        public string? StatusText { get; set; }

        [JsonPropertyName("duplicate_of_slots")]
        public int[]? DuplicateOfSlots { get; set; }
    }
}
