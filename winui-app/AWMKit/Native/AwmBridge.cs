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

    public readonly record struct EmbedEvidenceResult(
        double? SnrDb,
        string SnrStatus,
        string? SnrDetail
    );

    public readonly record struct AudioMediaCapabilitiesResult(
        string Backend,
        bool Eac3Decode,
        bool ContainerMp4,
        bool ContainerMkv,
        bool ContainerTs
    );

    public readonly record struct ProgressSnapshot(
        AwmProgressOperation Operation,
        AwmProgressPhase Phase,
        AwmProgressState State,
        bool Determinate,
        ulong CompletedUnits,
        ulong TotalUnits,
        uint StepIndex,
        uint StepTotal,
        ulong OpId,
        string PhaseLabel
    );

    private sealed class ProgressCallbackContext
    {
        public required Action<ProgressSnapshot> Handler { get; init; }
    }

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
        return EncodeMessage(tag, key, null);
    }

    /// <summary>
    /// Encodes a watermark message from tag and key with optional key slot.
    /// </summary>
    public static (byte[]? message, AwmError error) EncodeMessage(string tag, byte[] key, int? keySlot)
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
            int code = keySlot.HasValue
                ? AwmNative.awm_message_encode_with_slot(
                    version,
                    tag,
                    keyHandle.AddrOfPinnedObject(),
                    (nuint)key.Length,
                    (byte)Math.Clamp(keySlot.Value, 0, 31),
                    messageBuffer)
                : AwmNative.awm_message_encode(
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
    /// Decodes a watermark message without HMAC verification.
    /// </summary>
    public static (AWMResult? result, AwmError error) DecodeMessageUnverified(byte[] message)
    {
        if (message.Length != MessageLength)
        {
            return (null, AwmError.InvalidMessageLength);
        }

        var messageHandle = GCHandle.Alloc(message, GCHandleType.Pinned);
        var resultPtr = Marshal.AllocHGlobal(Marshal.SizeOf<AWMResult>());
        try
        {
            int code = AwmNative.awm_message_decode_unverified(
                messageHandle.AddrOfPinnedObject(),
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
            Marshal.FreeHGlobal(resultPtr);
        }
    }

    private static ProgressSnapshot ToProgressSnapshot(AWMProgressSnapshot native)
    {
        return new ProgressSnapshot(
            native.Operation,
            native.Phase,
            native.State,
            native.Determinate,
            native.CompletedUnits,
            native.TotalUnits,
            native.StepIndex,
            native.StepTotal,
            native.OpId,
            native.GetPhaseLabel()
        );
    }

    private static (AwmNative.AwmProgressCallback? callback, GCHandle contextHandle, bool enabled) TryInstallProgressCallback(
        IntPtr handle,
        Action<ProgressSnapshot>? onProgress)
    {
        if (onProgress is null)
        {
            return (null, default, false);
        }

        var context = new ProgressCallbackContext { Handler = onProgress };
        var contextHandle = GCHandle.Alloc(context);
        AwmNative.AwmProgressCallback callback = (snapshotPtr, userData) =>
        {
            if (snapshotPtr == IntPtr.Zero || userData == IntPtr.Zero)
            {
                return;
            }

            var callbackHandle = GCHandle.FromIntPtr(userData);
            if (callbackHandle.Target is not ProgressCallbackContext callbackContext)
            {
                return;
            }

            var nativeSnapshot = Marshal.PtrToStructure<AWMProgressSnapshot>(snapshotPtr);
            callbackContext.Handler(ToProgressSnapshot(nativeSnapshot));
        };

        try
        {
            int code = AwmNative.awm_audio_progress_set_callback(
                handle,
                callback,
                GCHandle.ToIntPtr(contextHandle));
            if (code != 0)
            {
                contextHandle.Free();
                return (null, default, false);
            }

            return (callback, contextHandle, true);
        }
        catch (EntryPointNotFoundException)
        {
            contextHandle.Free();
            return (null, default, false);
        }
    }

    private static void ClearProgressCallback(IntPtr handle)
    {
        try
        {
            AwmNative.awm_audio_progress_set_callback(handle, null, IntPtr.Zero);
            AwmNative.awm_audio_progress_clear(handle);
        }
        catch (EntryPointNotFoundException)
        {
            // Ignore when running on older native libraries.
        }
    }

    /// <summary>
    /// Embeds a watermark into an audio file.
    /// </summary>
    public static AwmError EmbedAudio(
        string inputPath,
        string outputPath,
        byte[] message,
        int strength = 10,
        Action<ProgressSnapshot>? onProgress = null)
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

        var nativeHandle = handle.DangerousGetHandle();
        var (progressCallback, progressContextHandle, progressEnabled) = TryInstallProgressCallback(nativeHandle, onProgress);
        AwmNative.awm_audio_set_strength(handle.DangerousGetHandle(), (byte)strength);

        var messageHandle = GCHandle.Alloc(message, GCHandleType.Pinned);
        try
        {
            int code = AwmNative.awm_audio_embed(
                nativeHandle,
                inputPath,
                outputPath,
                messageHandle.AddrOfPinnedObject());

            return (AwmError)code;
        }
        finally
        {
            messageHandle.Free();
            if (progressEnabled)
            {
                ClearProgressCallback(nativeHandle);
            }
            if (progressContextHandle.IsAllocated)
            {
                progressContextHandle.Free();
            }
            GC.KeepAlive(progressCallback);
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
        int strength = 10,
        Action<ProgressSnapshot>? onProgress = null)
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

        var nativeHandle = handle.DangerousGetHandle();
        var (progressCallback, progressContextHandle, progressEnabled) = TryInstallProgressCallback(nativeHandle, onProgress);
        AwmNative.awm_audio_set_strength(handle.DangerousGetHandle(), (byte)strength);

        var messageHandle = GCHandle.Alloc(message, GCHandleType.Pinned);
        try
        {
            int code = AwmNative.awm_audio_embed_multichannel(
                nativeHandle,
                inputPath,
                outputPath,
                messageHandle.AddrOfPinnedObject(),
                layout);

            return (AwmError)code;
        }
        catch (EntryPointNotFoundException)
        {
            return EmbedAudio(inputPath, outputPath, message, strength, onProgress);
        }
        finally
        {
            messageHandle.Free();
            if (progressEnabled)
            {
                ClearProgressCallback(nativeHandle);
            }
            if (progressContextHandle.IsAllocated)
            {
                progressContextHandle.Free();
            }
            GC.KeepAlive(progressCallback);
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
    public static (DetectAudioResult? result, AwmError error) DetectAudioDetailed(
        string inputPath,
        Action<ProgressSnapshot>? onProgress = null)
    {
        using var handle = AwmAudioHandle.CreateNew();
        if (handle.IsInvalid)
        {
            return (null, AwmError.AudiowmarkNotFound);
        }

        var nativeHandle = handle.DangerousGetHandle();
        var (progressCallback, progressContextHandle, progressEnabled) = TryInstallProgressCallback(nativeHandle, onProgress);
        var resultPtr = Marshal.AllocHGlobal(Marshal.SizeOf<AWMDetectResult>());
        try
        {
            int code = AwmNative.awm_audio_detect(
                nativeHandle,
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
            if (progressEnabled)
            {
                ClearProgressCallback(nativeHandle);
            }
            if (progressContextHandle.IsAllocated)
            {
                progressContextHandle.Free();
            }
            GC.KeepAlive(progressCallback);
        }
    }

    /// <summary>
    /// Detects a watermark with multichannel routing (returns best pair).
    /// </summary>
    public static (MultichannelDetectAudioResult? result, AwmError error) DetectAudioMultichannelDetailed(
        string inputPath,
        AwmChannelLayout layout,
        Action<ProgressSnapshot>? onProgress = null)
    {
        using var handle = AwmAudioHandle.CreateNew();
        if (handle.IsInvalid)
        {
            return (null, AwmError.AudiowmarkNotFound);
        }

        var nativeHandle = handle.DangerousGetHandle();
        var (progressCallback, progressContextHandle, progressEnabled) = TryInstallProgressCallback(nativeHandle, onProgress);
        var resultPtr = Marshal.AllocHGlobal(Marshal.SizeOf<AWMMultichannelDetectResult>());
        try
        {
            int code = AwmNative.awm_audio_detect_multichannel(
                nativeHandle,
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
            var (fallback, fallbackError) = DetectAudioDetailed(inputPath, onProgress);
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
            if (progressEnabled)
            {
                ClearProgressCallback(nativeHandle);
            }
            if (progressContextHandle.IsAllocated)
            {
                progressContextHandle.Free();
            }
            GC.KeepAlive(progressCallback);
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
    /// Records embed evidence and returns SNR analysis payload.
    /// </summary>
    public static (EmbedEvidenceResult? result, AwmError error) RecordEmbedEvidence(
        string inputPath,
        string outputPath,
        byte[] rawMessage,
        byte[] key,
        bool isForcedEmbed = false)
    {
        if (rawMessage.Length != MessageLength || key.Length == 0)
        {
            return (null, AwmError.InvalidMessageLength);
        }

        var messageHandle = GCHandle.Alloc(rawMessage, GCHandleType.Pinned);
        var keyHandle = GCHandle.Alloc(key, GCHandleType.Pinned);
        var resultPtr = Marshal.AllocHGlobal(Marshal.SizeOf<AWMEmbedEvidenceResult>());
        try
        {
            try
            {
                int code = AwmNative.awm_evidence_record_embed_file_ex(
                    inputPath,
                    outputPath,
                    messageHandle.AddrOfPinnedObject(),
                    keyHandle.AddrOfPinnedObject(),
                    (nuint)key.Length,
                    isForcedEmbed,
                    resultPtr);
                var error = (AwmError)code;
                if (error != AwmError.Ok)
                {
                    return (null, error);
                }

                var native = Marshal.PtrToStructure<AWMEmbedEvidenceResult>(resultPtr);
                var payload = new EmbedEvidenceResult(
                    native.HasSnrDb ? native.SnrDb : null,
                    string.IsNullOrWhiteSpace(native.GetSnrStatus()) ? "unavailable" : native.GetSnrStatus(),
                    native.GetSnrDetail());
                return (payload, AwmError.Ok);
            }
            catch (EntryPointNotFoundException)
            {
                var fallback = RecordEvidenceFile(outputPath, rawMessage, key, isForcedEmbed);
                if (fallback != AwmError.Ok)
                {
                    return (null, fallback);
                }

                return (new EmbedEvidenceResult(null, "unavailable", "legacy_ffi"), AwmError.Ok);
            }
        }
        finally
        {
            Marshal.FreeHGlobal(resultPtr);
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
    /// Gets FFmpeg media decode capabilities.
    /// </summary>
    public static (AudioMediaCapabilitiesResult? caps, AwmError error) GetAudioMediaCapabilities()
    {
        using var handle = AwmAudioHandle.CreateNew();
        if (handle.IsInvalid)
        {
            return (null, AwmError.AudiowmarkNotFound);
        }

        var resultPtr = Marshal.AllocHGlobal(Marshal.SizeOf<AWMAudioMediaCapabilities>());
        try
        {
            int code = AwmNative.awm_audio_media_capabilities(handle.DangerousGetHandle(), resultPtr);
            var error = (AwmError)code;
            if (error != AwmError.Ok)
            {
                return (null, error);
            }

            var native = Marshal.PtrToStructure<AWMAudioMediaCapabilities>(resultPtr);
            var caps = new AudioMediaCapabilitiesResult(
                native.GetBackend(),
                native.Eac3Decode,
                native.ContainerMp4,
                native.ContainerMkv,
                native.ContainerTs
            );
            return (caps, AwmError.Ok);
        }
        finally
        {
            Marshal.FreeHGlobal(resultPtr);
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
