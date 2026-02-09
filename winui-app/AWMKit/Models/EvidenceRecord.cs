using System;

namespace AWMKit.Models;

/// <summary>
/// Represents an audio evidence record stored in the audio_evidence table.
/// </summary>
public sealed class EvidenceRecord
{
    /// <summary>
    /// Unique record ID.
    /// </summary>
    public int Id { get; init; }

    /// <summary>
    /// Full path to the audio file.
    /// </summary>
    public required string FilePath { get; init; }

    /// <summary>
    /// SHA-256 hash of the audio file.
    /// </summary>
    public required string FileHash { get; init; }

    /// <summary>
    /// 16-byte watermark message (hex-encoded).
    /// </summary>
    public required string Message { get; init; }

    /// <summary>
    /// Detection pattern quality string (e.g., "5.2").
    /// </summary>
    public required string Pattern { get; init; }

    /// <summary>
    /// Decoded tag from the message (e.g., "ABCD1234").
    /// </summary>
    public required string Tag { get; init; }

    /// <summary>
    /// Timestamp when the evidence was recorded (UTC).
    /// </summary>
    public DateTime CreatedAt { get; init; }
}
