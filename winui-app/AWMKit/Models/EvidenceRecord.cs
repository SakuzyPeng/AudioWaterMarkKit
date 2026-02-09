using System;

namespace AWMKit.Models;

/// <summary>
/// Represents one row from audio_evidence.
/// </summary>
public sealed class EvidenceRecord
{
    public long Id { get; init; }
    public long CreatedAt { get; init; }
    public required string FilePath { get; init; }
    public required string Tag { get; init; }
    public required string Identity { get; init; }
    public int Version { get; init; }
    public int KeySlot { get; init; }
    public long TimestampMinutes { get; init; }
    public required string MessageHex { get; init; }
    public int SampleRate { get; init; }
    public int Channels { get; init; }
    public long SampleCount { get; init; }
    public required string PcmSha256 { get; init; }
    public required byte[] ChromaprintBlob { get; init; }
    public int FingerprintLen { get; init; }
    public int FpConfigId { get; init; }

    public DateTime CreatedAtDateTime => DateTimeOffset.FromUnixTimeSeconds(CreatedAt).UtcDateTime;

    // Compatibility shims for existing WinUI code paths that still reference old names.
    public string FileHash => PcmSha256;
    public string Message => MessageHex;
    public string Pattern => "-";
}
