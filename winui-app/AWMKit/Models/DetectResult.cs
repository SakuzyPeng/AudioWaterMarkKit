using AWMKit.Native;
using System.Globalization;

namespace AWMKit.Models;

/// <summary>
/// Result model for audio watermark detection operation.
/// </summary>
public sealed class DetectResult
{
    public required string FilePath { get; init; }
    public bool Success { get; init; }
    public string? Tag { get; init; }
    public string? Identity { get; init; }
    public byte? KeySlot { get; init; }
    public uint? TimestampMinutes { get; init; }
    public string? Pattern { get; init; }
    public uint? BitErrors { get; init; }
    public float? DetectScore { get; init; }
    public AwmCloneCheckKind? CloneCheck { get; init; }
    public double? CloneScore { get; init; }
    public float? CloneMatchSeconds { get; init; }
    public long? CloneEvidenceId { get; init; }
    public string? CloneReason { get; init; }
    public byte[]? Message { get; init; }
    public AwmError? Error { get; init; }
    public string? ErrorMessage { get; init; }

    public string DetectScoreText =>
        DetectScore.HasValue ? DetectScore.Value.ToString("0.###", CultureInfo.InvariantCulture) : string.Empty;

    public string CloneCheckText =>
        CloneCheck.HasValue ? CloneCheck.Value.ToString().ToLowerInvariant() : string.Empty;

    public string CloneScoreText
    {
        get
        {
            if (!CloneScore.HasValue)
            {
                return string.Empty;
            }

            if (CloneMatchSeconds.HasValue)
            {
                return $"{CloneScore.Value.ToString("0.###", CultureInfo.InvariantCulture)} / {CloneMatchSeconds.Value.ToString("0.#", CultureInfo.InvariantCulture)}s";
            }

            return CloneScore.Value.ToString("0.###", CultureInfo.InvariantCulture);
        }
    }

    // Compatibility properties for current WinUI bindings.
    public string? DisplayName => null;
    public string? FileHash => null;
}
