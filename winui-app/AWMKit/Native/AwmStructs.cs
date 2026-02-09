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
