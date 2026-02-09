using System;
using System.Runtime.InteropServices;
using System.Text;

namespace AWMKit.Native;

/// <summary>
/// High-level bridge for AWMKit FFI operations.
/// Handles marshalling, memory management, and error checking.
/// </summary>
public static class AwmBridge
{
    private const int MessageLength = 16;
    private const int TagLength = 8;
    private const int IdentityMaxLength = 7;

    /// <summary>
    /// Gets the current message format version.
    /// </summary>
    public static byte GetCurrentVersion()
    {
        return AwmNative.awm_current_version();
    }

    /// <summary>
    /// Creates a new tag from an identity string.
    /// </summary>
    /// <param name="identity">Identity string (will be padded and checksummed)</param>
    /// <returns>(tag, error code)</returns>
    public static (string? tag, AwmError error) CreateTag(string identity)
    {
        var buffer = Marshal.AllocHGlobal(9); // 8 chars + null
        try
        {
            int code = AwmNative.awm_tag_new(identity, buffer);
            if (code == 0)
            {
                var tag = Marshal.PtrToStringUTF8(buffer);
                return (tag, AwmError.Ok);
            }
            return (null, (AwmError)code);
        }
        finally
        {
            Marshal.FreeHGlobal(buffer);
        }
    }

    /// <summary>
    /// Verifies a tag's checksum.
    /// </summary>
    public static bool VerifyTag(string tag)
    {
        return AwmNative.awm_tag_verify(tag);
    }

    /// <summary>
    /// Extracts the identity portion from a tag.
    /// </summary>
    public static (string? identity, AwmError error) GetTagIdentity(string tag)
    {
        var buffer = Marshal.AllocHGlobal(8); // 7 chars + null
        try
        {
            int code = AwmNative.awm_tag_identity(tag, buffer);
            if (code == 0)
            {
                var identity = Marshal.PtrToStringUTF8(buffer);
                return (identity, AwmError.Ok);
            }
            return (null, (AwmError)code);
        }
        finally
        {
            Marshal.FreeHGlobal(buffer);
        }
    }

    /// <summary>
    /// Generates a random tag suggestion from a username.
    /// </summary>
    public static (string? tag, AwmError error) SuggestTag(string username)
    {
        var buffer = Marshal.AllocHGlobal(9); // 8 chars + null
        try
        {
            int code = AwmNative.awm_tag_suggest(username, buffer);
            if (code == 0)
            {
                var tag = Marshal.PtrToStringUTF8(buffer);
                return (tag, AwmError.Ok);
            }
            return (null, (AwmError)code);
        }
        finally
        {
            Marshal.FreeHGlobal(buffer);
        }
    }

    /// <summary>
    /// Encodes a watermark message from tag and key.
    /// </summary>
    public static (byte[]? message, AwmError error) EncodeMessage(string tag, byte[] key)
    {
        if (key.Length != 32)
        {
            return (null, AwmError.InvalidMessageLength);
        }

        var messageBuffer = Marshal.AllocHGlobal(MessageLength);
        var keyHandle = GCHandle.Alloc(key, GCHandleType.Pinned);
        try
        {
            byte version = GetCurrentVersion();
            int code = AwmNative.awm_message_encode(
                version,
                tag,
                keyHandle.AddrOfPinnedObject(),
                (nuint)key.Length,
                messageBuffer);

            if (code == 0)
            {
                var message = new byte[MessageLength];
                Marshal.Copy(messageBuffer, message, 0, MessageLength);
                return (message, AwmError.Ok);
            }
            return (null, (AwmError)code);
        }
        finally
        {
            keyHandle.Free();
            Marshal.FreeHGlobal(messageBuffer);
        }
    }

    /// <summary>
    /// Decodes a watermark message to extract tag and metadata.
    /// </summary>
    public static (AWMResult? result, AwmError error) DecodeMessage(byte[] message, byte[] key)
    {
        if (message.Length != MessageLength || key.Length != 32)
        {
            return (null, AwmError.InvalidMessageLength);
        }

        var messageHandle = GCHandle.Alloc(message, GCHandleType.Pinned);
        var keyHandle = GCHandle.Alloc(key, GCHandleType.Pinned);
        var resultPtr = Marshal.AllocHGlobal(Marshal.SizeOf<AWMResult>());
        try
        {
            int code = AwmNative.awm_message_decode(
                messageHandle.AddrOfPinnedObject(),
                keyHandle.AddrOfPinnedObject(),
                (nuint)key.Length,
                resultPtr);

            if (code == 0)
            {
                var result = Marshal.PtrToStructure<AWMResult>(resultPtr);
                return (result, AwmError.Ok);
            }
            return (null, (AwmError)code);
        }
        finally
        {
            messageHandle.Free();
            keyHandle.Free();
            Marshal.FreeHGlobal(resultPtr);
        }
    }

    /// <summary>
    /// Embeds a watermark into an audio file.
    /// </summary>
    public static AwmError EmbedAudio(string inputPath, string outputPath, byte[] message, int strength = 10)
    {
        if (message.Length != MessageLength)
        {
            return AwmError.InvalidMessageLength;
        }

        using var handle = AwmAudioHandle.CreateNew();
        if (handle.IsInvalid)
        {
            return AwmError.AudiowmarkNotFound;
        }

        AwmNative.awm_audio_set_strength(handle.DangerousGetHandle(), (byte)strength);

        var messageHandle = GCHandle.Alloc(message, GCHandleType.Pinned);
        try
        {
            int code = AwmNative.awm_audio_embed(
                handle.DangerousGetHandle(),
                inputPath,
                outputPath,
                messageHandle.AddrOfPinnedObject());

            return (AwmError)code;
        }
        finally
        {
            messageHandle.Free();
        }
    }

    /// <summary>
    /// Detects a watermark from an audio file.
    /// </summary>
    public static (byte[]? message, string pattern, AwmError error) DetectAudio(string inputPath)
    {
        using var handle = AwmAudioHandle.CreateNew();
        if (handle.IsInvalid)
        {
            return (null, string.Empty, AwmError.AudiowmarkNotFound);
        }

        var resultPtr = Marshal.AllocHGlobal(Marshal.SizeOf<AWMDetectResult>());
        try
        {
            int code = AwmNative.awm_audio_detect(
                handle.DangerousGetHandle(),
                inputPath,
                resultPtr);

            if (code == 0)
            {
                var result = Marshal.PtrToStructure<AWMDetectResult>(resultPtr);
                var message = new byte[MessageLength];
                Array.Copy(result.RawMessage, message, MessageLength);
                string pattern = result.GetPattern();
                return (message, pattern, AwmError.Ok);
            }
            return (null, string.Empty, (AwmError)code);
        }
        finally
        {
            Marshal.FreeHGlobal(resultPtr);
        }
    }

    /// <summary>
    /// Checks if the audiowmark binary is available.
    /// </summary>
    public static bool IsAudioAvailable()
    {
        using var handle = AwmAudioHandle.CreateNew();
        if (handle.IsInvalid)
        {
            return false;
        }

        return AwmNative.awm_audio_is_available(handle.DangerousGetHandle());
    }

    /// <summary>
    /// Gets the path to the audiowmark binary.
    /// </summary>
    public static (string? path, AwmError error) GetAudioBinaryPath()
    {
        using var handle = AwmAudioHandle.CreateNew();
        if (handle.IsInvalid)
        {
            return (null, AwmError.AudiowmarkNotFound);
        }

        var buffer = Marshal.AllocHGlobal(4096);
        try
        {
            int code = AwmNative.awm_audio_binary_path(handle.DangerousGetHandle(), buffer, 4096);
            if (code == 0)
            {
                var path = Marshal.PtrToStringUTF8(buffer);
                return (path, AwmError.Ok);
            }
            return (null, (AwmError)code);
        }
        finally
        {
            Marshal.FreeHGlobal(buffer);
        }
    }
}
