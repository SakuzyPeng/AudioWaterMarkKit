using System;
using System.Text;
using System.Runtime.InteropServices;

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
}
