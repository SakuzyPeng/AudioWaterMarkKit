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

    public readonly record struct DetectAudioResult(
        byte[] RawMessage,
        string Pattern,
        uint BitErrors,
        float? DetectScore
    );

    public readonly record struct MultichannelDetectAudioResult(
        byte[] RawMessage,
        uint BitErrors,
        uint PairCount,
        string Pattern,
        float? DetectScore
    );

    public readonly record struct CloneCheckResult(
        AwmCloneCheckKind Kind,
        double? Score,
        float? MatchSeconds,
        long? EvidenceId,
        string? Reason
    );

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
    /// Embeds a watermark with multichannel routing.
    /// </summary>
    public static AwmError EmbedAudioMultichannel(
        string inputPath,
        string outputPath,
        byte[] message,
        AwmChannelLayout layout,
        int strength = 10)
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
            int code = AwmNative.awm_audio_embed_multichannel(
                handle.DangerousGetHandle(),
                inputPath,
                outputPath,
                messageHandle.AddrOfPinnedObject(),
                layout);

            return (AwmError)code;
        }
        catch (EntryPointNotFoundException)
        {
            return EmbedAudio(inputPath, outputPath, message, strength);
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
        var (result, error) = DetectAudioDetailed(inputPath);
        if (error != AwmError.Ok || result is null)
        {
            return (null, string.Empty, error);
        }

        return (result.Value.RawMessage, result.Value.Pattern, AwmError.Ok);
    }

    /// <summary>
    /// Detects a watermark from an audio file (extended result).
    /// </summary>
    public static (DetectAudioResult? result, AwmError error) DetectAudioDetailed(string inputPath)
    {
        using var handle = AwmAudioHandle.CreateNew();
        if (handle.IsInvalid)
        {
            return (null, AwmError.AudiowmarkNotFound);
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
                return (new DetectAudioResult(
                    message,
                    pattern,
                    result.BitErrors,
                    result.HasDetectScore ? result.DetectScore : null
                ), AwmError.Ok);
            }
            return (null, (AwmError)code);
        }
        finally
        {
            Marshal.FreeHGlobal(resultPtr);
        }
    }

    /// <summary>
    /// Detects a watermark with multichannel routing (returns best pair).
    /// </summary>
    public static (MultichannelDetectAudioResult? result, AwmError error) DetectAudioMultichannelDetailed(
        string inputPath,
        AwmChannelLayout layout)
    {
        using var handle = AwmAudioHandle.CreateNew();
        if (handle.IsInvalid)
        {
            return (null, AwmError.AudiowmarkNotFound);
        }

        var resultPtr = Marshal.AllocHGlobal(Marshal.SizeOf<AWMMultichannelDetectResult>());
        try
        {
            int code = AwmNative.awm_audio_detect_multichannel(
                handle.DangerousGetHandle(),
                inputPath,
                layout,
                resultPtr);

            if (code == 0)
            {
                var result = Marshal.PtrToStructure<AWMMultichannelDetectResult>(resultPtr);
                if (!result.HasBest || result.BestRawMessage is null)
                {
                    return (null, AwmError.NoWatermarkFound);
                }

                var message = new byte[MessageLength];
                Array.Copy(result.BestRawMessage, message, MessageLength);

                return (new MultichannelDetectAudioResult(
                    message,
                    result.BestBitErrors,
                    result.PairCount,
                    result.GetBestPattern(),
                    result.HasBestDetectScore ? result.BestDetectScore : null
                ), AwmError.Ok);
            }

            return (null, (AwmError)code);
        }
        catch (EntryPointNotFoundException)
        {
            var (fallback, fallbackError) = DetectAudioDetailed(inputPath);
            if (fallbackError != AwmError.Ok || fallback is null)
            {
                return (null, fallbackError);
            }

            return (new MultichannelDetectAudioResult(
                fallback.Value.RawMessage,
                fallback.Value.BitErrors,
                1,
                fallback.Value.Pattern,
                fallback.Value.DetectScore
            ), AwmError.Ok);
        }
        finally
        {
            Marshal.FreeHGlobal(resultPtr);
        }
    }

    /// <summary>
    /// Returns channel count for a layout.
    /// </summary>
    public static uint GetLayoutChannels(AwmChannelLayout layout)
    {
        try
        {
            return AwmNative.awm_channel_layout_channels(layout);
        }
        catch (EntryPointNotFoundException)
        {
            return layout switch
            {
                AwmChannelLayout.Stereo => 2,
                AwmChannelLayout.Surround51 => 6,
                AwmChannelLayout.Surround512 => 8,
                AwmChannelLayout.Surround71 => 8,
                AwmChannelLayout.Surround714 => 12,
                AwmChannelLayout.Surround916 => 16,
                _ => 0,
            };
        }
    }

    /// <summary>
    /// Runs clone-check for decoded identity/key slot.
    /// </summary>
    public static (CloneCheckResult? result, AwmError error) CloneCheckForFile(string inputPath, string identity, byte keySlot)
    {
        var resultPtr = Marshal.AllocHGlobal(Marshal.SizeOf<AWMCloneCheckResult>());
        try
        {
            int code = AwmNative.awm_clone_check_for_file(inputPath, identity, keySlot, resultPtr);
            if (code != 0)
            {
                return (null, (AwmError)code);
            }

            var result = Marshal.PtrToStructure<AWMCloneCheckResult>(resultPtr);
            var reason = result.GetReason();
            return (new CloneCheckResult(
                result.CloneKind,
                result.HasScore ? result.Score : null,
                result.HasMatchSeconds ? result.MatchSeconds : null,
                result.HasEvidenceId ? result.EvidenceId : null,
                string.IsNullOrWhiteSpace(reason) ? null : reason
            ), AwmError.Ok);
        }
        finally
        {
            Marshal.FreeHGlobal(resultPtr);
        }
    }

    /// <summary>
    /// Records evidence for an embedded output file.
    /// </summary>
    public static AwmError RecordEvidenceFile(string filePath, byte[] rawMessage, byte[] key, bool isForcedEmbed = false)
    {
        if (rawMessage.Length != MessageLength || key.Length == 0)
        {
            return AwmError.InvalidMessageLength;
        }

        var messageHandle = GCHandle.Alloc(rawMessage, GCHandleType.Pinned);
        var keyHandle = GCHandle.Alloc(key, GCHandleType.Pinned);
        try
        {
            try
            {
                int code = AwmNative.awm_evidence_record_file_ex(
                    filePath,
                    messageHandle.AddrOfPinnedObject(),
                    keyHandle.AddrOfPinnedObject(),
                    (nuint)key.Length,
                    isForcedEmbed);
                return (AwmError)code;
            }
            catch (EntryPointNotFoundException)
            {
                // Compatibility fallback for older native libraries without *_ex.
                int code = AwmNative.awm_evidence_record_file(
                    filePath,
                    messageHandle.AddrOfPinnedObject(),
                    keyHandle.AddrOfPinnedObject(),
                    (nuint)key.Length);
                return (AwmError)code;
            }
        }
        finally
        {
            keyHandle.Free();
            messageHandle.Free();
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

    /// <summary>
    /// Gets persisted UI language override ("zh-CN" | "en-US"), null if unset.
    /// </summary>
    public static (string? language, AwmError error) GetUiLanguage()
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

            int first = AwmNative.awm_ui_language_get(IntPtr.Zero, 0, requiredPtr);
            var firstError = (AwmError)first;
            if (firstError != AwmError.Ok)
            {
                return (null, firstError);
            }

            var required = IntPtr.Size == 8
                ? (nuint)Marshal.ReadInt64(requiredPtr)
                : (nuint)Marshal.ReadInt32(requiredPtr);
            if (required == 0)
            {
                return (null, AwmError.Ok);
            }

            var buffer = Marshal.AllocHGlobal((int)required);
            try
            {
                int second = AwmNative.awm_ui_language_get(buffer, required, requiredPtr);
                var secondError = (AwmError)second;
                if (secondError != AwmError.Ok)
                {
                    return (null, secondError);
                }

                var value = Marshal.PtrToStringUTF8(buffer);
                return (string.IsNullOrWhiteSpace(value) ? null : value, AwmError.Ok);
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

    /// <summary>
    /// Sets persisted UI language override. Null/empty clears override.
    /// </summary>
    public static AwmError SetUiLanguage(string? language)
    {
        var normalized = string.IsNullOrWhiteSpace(language) ? null : language.Trim();
        int code = AwmNative.awm_ui_language_set(normalized);
        return (AwmError)code;
    }
}
