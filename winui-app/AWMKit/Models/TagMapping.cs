using System;

namespace AWMKit.Models;

/// <summary>
/// Represents a user-to-tag mapping stored in the tag_mappings table.
/// </summary>
public sealed class TagMapping
{
    /// <summary>
    /// User identity (e.g., "alice@example.com").
    /// </summary>
    public required string Identity { get; init; }

    /// <summary>
    /// 8-character tag (e.g., "ABCD1234").
    /// </summary>
    public required string Tag { get; init; }

    /// <summary>
    /// Optional display name for the user.
    /// </summary>
    public string? DisplayName { get; init; }

    /// <summary>
    /// Timestamp when the mapping was created (UTC).
    /// </summary>
    public DateTime CreatedAt { get; init; }

    /// <summary>
    /// Timestamp when the mapping was last updated (UTC).
    /// </summary>
    public DateTime UpdatedAt { get; init; }
}
