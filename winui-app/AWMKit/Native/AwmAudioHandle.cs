using System;
using Microsoft.Win32.SafeHandles;

namespace AWMKit.Native;

/// <summary>
/// SafeHandle wrapper for AWMAudioHandle* from Rust FFI.
/// Automatically calls awm_audio_free on disposal.
/// </summary>
internal sealed class AwmAudioHandle : SafeHandleZeroOrMinusOneIsInvalid
{
    public AwmAudioHandle() : base(ownsHandle: true)
    {
    }

    private AwmAudioHandle(IntPtr existingHandle, bool ownsHandle) : base(ownsHandle)
    {
        SetHandle(existingHandle);
    }

    public static AwmAudioHandle CreateNew()
    {
        var handle = AwmNative.awm_audio_new();
        return new AwmAudioHandle(handle, true);
    }

    public static AwmAudioHandle CreateWithBinary(string binaryPath)
    {
        var handle = AwmNative.awm_audio_new_with_binary(binaryPath);
        return new AwmAudioHandle(handle, true);
    }

    protected override bool ReleaseHandle()
    {
        if (!IsInvalid)
        {
            AwmNative.awm_audio_free(handle);
        }
        return true;
    }
}
