namespace AWMKit.Models;

/// <summary>
/// Request model for audio watermark embedding operation.
/// </summary>
public sealed class EmbedRequest
{
    /// <summary>
    /// User identity for key lookup (e.g., "alice@example.com").
    /// </summary>
    public required string Identity { get; init; }

    /// <summary>
    /// Source audio file paths to embed.
    /// </summary>
    public required string[] InputFiles { get; init; }

    /// <summary>
    /// Output directory for watermarked files.
    /// </summary>
    public required string OutputDirectory { get; init; }

    /// <summary>
    /// Embedding strength (1-20, default: 10).
    /// </summary>
    public int Strength { get; init; } = 10;

    /// <summary>
    /// Whether to overwrite existing files.
    /// </summary>
    public bool Overwrite { get; init; }
}
