using System;
using System.Runtime.InteropServices;

namespace AWMKit.Native;

/// <summary>
/// Bridge for key management operations using Windows DPAPI (via Rust keystore).
/// NOTE: Rust KeyStore is GLOBAL - only ONE key is stored per system (no per-user identity).
/// All keys are 32 bytes (256-bit HMAC-SHA256 keys).
/// </summary>
public static class AwmKeyBridge
{
    private const int KeySize = 32;

    /// <summary>
    /// Checks if a key exists in the system keystore.
    /// NOTE: This checks for a GLOBAL key, not per-user.
    /// </summary>
    public static bool KeyExists()
    {
        return AwmNative.awm_key_exists();
    }

    /// <summary>
    /// Loads the global key from the system keystore.
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
    /// Generates a new random key and saves it to the system keystore.
    /// WARNING: This will REPLACE any existing global key.
    /// </summary>
    /// <returns>(generated key bytes, error code). Key is null if error occurred.</returns>
    public static (byte[]? key, AwmError error) GenerateAndSaveKey()
    {
        var keyBuffer = new byte[KeySize];
        var handle = GCHandle.Alloc(keyBuffer, GCHandleType.Pinned);
        try
        {
            int code = AwmNative.awm_key_generate_and_save(handle.AddrOfPinnedObject(), (nuint)KeySize);
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
    /// Deletes the global key from the system keystore.
    /// </summary>
    /// <returns>Error code (0 = success)</returns>
    public static AwmError DeleteKey()
    {
        int code = AwmNative.awm_key_delete();
        return (AwmError)code;
    }

    /// <summary>
    /// Gets or creates the global key.
    /// If key exists, loads it; otherwise generates a new one.
    /// </summary>
    /// <returns>(key bytes, was newly generated, error code)</returns>
    public static (byte[]? key, bool isNew, AwmError error)  GetOrCreateKey()
    {
        if (KeyExists())
        {
            var (key, err) = LoadKey();
            return (key, false, err);
        }
        else
        {
            var (key, err) = GenerateAndSaveKey();
            return (key, true, err);
        }
    }
}
