using System.Runtime.InteropServices;

namespace AWMKit.Native;

/// <summary>FFI error codes matching AWMError in Rust.</summary>
public enum AwmError
{
    Ok = 0,
    Success = 0, // Alias for Ok
    InvalidTag = -1,
    InvalidMessageLength = -2,
    HmacMismatch = -3,
    NullPointer = -4,
    InvalidUtf8 = -5,
    ChecksumMismatch = -6,
    AudiowmarkNotFound = -7,
    AudiowmarkExec = -8,
    NoWatermarkFound = -9,
    KeyAlreadyExists = -10,
    InvalidOutputFormat = -11,
}

/// <summary>Multichannel layout enum matching AWMChannelLayout in C header.</summary>
public enum AwmChannelLayout : int
{
    Stereo = 0,
    Surround51 = 1,
    Surround512 = 2,
    Surround71 = 3,
    Surround714 = 4,
    Surround916 = 5,
    Auto = -1,
}

/// <summary>Decoded watermark message.</summary>
[StructLayout(LayoutKind.Sequential)]
public struct AWMResult
{
    public byte Version;
    public ulong TimestampUtc;
    public uint TimestampMinutes;
    public byte KeySlot;

    [MarshalAs(UnmanagedType.ByValArray, SizeConst = 9)]
    public byte[] Tag;

    [MarshalAs(UnmanagedType.ByValArray, SizeConst = 8)]
    public byte[] Identity;

    public string GetTag() => GetNullTerminatedString(Tag);
    public string GetIdentity() => GetNullTerminatedString(Identity);

    private static string GetNullTerminatedString(byte[] bytes)
    {
        int len = Array.IndexOf(bytes, (byte)0);
        if (len < 0) len = bytes.Length;
        return System.Text.Encoding.UTF8.GetString(bytes, 0, len);
    }
}

/// <summary>Audio detection result.</summary>
[StructLayout(LayoutKind.Sequential)]
public struct AWMDetectResult
{
    [MarshalAs(UnmanagedType.U1)]
    public bool Found;

    [MarshalAs(UnmanagedType.ByValArray, SizeConst = 16)]
    public byte[] RawMessage;

    [MarshalAs(UnmanagedType.ByValArray, SizeConst = 16)]
    public byte[] Pattern;

    [MarshalAs(UnmanagedType.U1)]
    public bool HasDetectScore;

    public float DetectScore;
    public uint BitErrors;

    public string GetPattern()
    {
        int len = Array.IndexOf(Pattern, (byte)0);
        if (len < 0) len = Pattern.Length;
        return System.Text.Encoding.UTF8.GetString(Pattern, 0, len);
    }
}

[StructLayout(LayoutKind.Sequential)]
public struct AWMAudioMediaCapabilities
{
    [MarshalAs(UnmanagedType.ByValArray, SizeConst = 16)]
    public byte[] Backend;

    [MarshalAs(UnmanagedType.U1)]
    public bool Eac3Decode;

    [MarshalAs(UnmanagedType.U1)]
    public bool ContainerMp4;

    [MarshalAs(UnmanagedType.U1)]
    public bool ContainerMkv;

    [MarshalAs(UnmanagedType.U1)]
    public bool ContainerTs;

    public string GetBackend()
    {
        int len = Array.IndexOf(Backend, (byte)0);
        if (len < 0) len = Backend.Length;
        return System.Text.Encoding.UTF8.GetString(Backend, 0, len);
    }
}

/// <summary>Single multichannel pair detection result.</summary>
[StructLayout(LayoutKind.Sequential)]
public struct AWMPairResult
{
    public uint PairIndex;

    [MarshalAs(UnmanagedType.U1)]
    public bool Found;

    [MarshalAs(UnmanagedType.ByValArray, SizeConst = 16)]
    public byte[] RawMessage;

    public uint BitErrors;
}

/// <summary>Multichannel detect result with best pair summary.</summary>
[StructLayout(LayoutKind.Sequential)]
public struct AWMMultichannelDetectResult
{
    public uint PairCount;

    [MarshalAs(UnmanagedType.ByValArray, SizeConst = 8)]
    public AWMPairResult[] Pairs;

    [MarshalAs(UnmanagedType.U1)]
    public bool HasBest;

    [MarshalAs(UnmanagedType.ByValArray, SizeConst = 16)]
    public byte[] BestRawMessage;

    [MarshalAs(UnmanagedType.ByValArray, SizeConst = 16)]
    public byte[] BestPattern;

    [MarshalAs(UnmanagedType.U1)]
    public bool HasBestDetectScore;

    public float BestDetectScore;

    public uint BestBitErrors;

    public string GetBestPattern()
    {
        int len = Array.IndexOf(BestPattern, (byte)0);
        if (len < 0) len = BestPattern.Length;
        return System.Text.Encoding.UTF8.GetString(BestPattern, 0, len);
    }
}

/// <summary>Clone check result kind.</summary>
public enum AwmCloneCheckKind
{
    Exact = 0,
    Likely = 1,
    Suspect = 2,
    Unavailable = 3,
}

/// <summary>Clone check result.</summary>
[StructLayout(LayoutKind.Sequential)]
public struct AWMCloneCheckResult
{
    public int Kind;

    [MarshalAs(UnmanagedType.U1)]
    public bool HasScore;
    public double Score;

    [MarshalAs(UnmanagedType.U1)]
    public bool HasMatchSeconds;
    public float MatchSeconds;

    [MarshalAs(UnmanagedType.U1)]
    public bool HasEvidenceId;
    public long EvidenceId;

    [MarshalAs(UnmanagedType.ByValArray, SizeConst = 128)]
    public byte[] Reason;

    public AwmCloneCheckKind CloneKind => (AwmCloneCheckKind)Kind;

    public string GetReason()
    {
        int len = Array.IndexOf(Reason, (byte)0);
        if (len < 0) len = Reason.Length;
        return System.Text.Encoding.UTF8.GetString(Reason, 0, len);
    }
}

/// <summary>Embed evidence record result with SNR.</summary>
[StructLayout(LayoutKind.Sequential)]
public struct AWMEmbedEvidenceResult
{
    [MarshalAs(UnmanagedType.U1)]
    public bool HasSnrDb;

    public double SnrDb;

    [MarshalAs(UnmanagedType.ByValArray, SizeConst = 16)]
    public byte[] SnrStatus;

    [MarshalAs(UnmanagedType.ByValArray, SizeConst = 128)]
    public byte[] SnrDetail;

    public string GetSnrStatus()
    {
        int len = Array.IndexOf(SnrStatus, (byte)0);
        if (len < 0) len = SnrStatus.Length;
        return System.Text.Encoding.UTF8.GetString(SnrStatus, 0, len);
    }

    public string? GetSnrDetail()
    {
        int len = Array.IndexOf(SnrDetail, (byte)0);
        if (len < 0) len = SnrDetail.Length;
        if (len == 0) return null;
        return System.Text.Encoding.UTF8.GetString(SnrDetail, 0, len);
    }
}
