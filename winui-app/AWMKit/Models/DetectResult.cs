using AWMKit.Native;

namespace AWMKit.Models;

/// <summary>
/// Result model for audio watermark detection operation.
/// </summary>
public sealed class DetectResult
{
    /// <summary>
    /// Audio file path that was analyzed.
    /// </summary>
    public required string FilePath { get; init; }

    /// <summary>
    /// SHA-256 hash of the audio file.
    /// </summary>
    public required string FileHash { get; init; }

    /// <summary>
    /// Detection success status.
    /// </summary>
    public bool Success { get; init; }

    /// <summary>
    /// Detected tag (null if detection failed).
    /// </summary>
    public string? Tag { get; init; }

    /// <summary>
    /// User identity from tag mapping (null if not found).
    /// </summary>
    public string? Identity { get; init; }

    /// <summary>
    /// User display name (null if not found).
    /// </summary>
    public string? DisplayName { get; init; }

    /// <summary>
    /// Detection pattern quality (null if detection failed).
    /// </summary>
    public string? Pattern { get; init; }

    /// <summary>
    /// Raw 16-byte message (null if detection failed).
    /// </summary>
    public byte[]? Message { get; init; }

    /// <summary>
    /// Error code if detection failed.
    /// </summary>
    public AwmError? Error { get; init; }

    /// <summary>
    /// Error message if detection failed.
    /// </summary>
    public string? ErrorMessage { get; init; }
}
