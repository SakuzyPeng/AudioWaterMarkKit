using System;

namespace AWMKit.Models;

/// <summary>
/// Structured detect result record aligned with macOS DetectRecord.
/// </summary>
public sealed class DetectRecord
{
    public Guid Id { get; init; } = Guid.NewGuid();
    public required string FilePath { get; init; }
    public required string Status { get; init; }
    public string? Tag { get; init; }
    public string? Identity { get; init; }
    public byte? Version { get; init; }
    public uint? TimestampMinutes { get; init; }
    public ulong? TimestampUtc { get; init; }
    public byte? KeySlot { get; init; }
    public string? Pattern { get; init; }
    public float? DetectScore { get; init; }
    public uint? BitErrors { get; init; }
    public bool? MatchFound { get; init; }
    public string? CloneCheck { get; init; }
    public double? CloneScore { get; init; }
    public float? CloneMatchSeconds { get; init; }
    public string? CloneReason { get; init; }
    public string? Error { get; init; }
    public string? Verification { get; init; }
    public DateTime Timestamp { get; init; } = DateTime.Now;
}
