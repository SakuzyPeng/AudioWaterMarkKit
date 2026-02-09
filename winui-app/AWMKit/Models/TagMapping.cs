using System;

namespace AWMKit.Models;

/// <summary>
/// Represents a username-to-tag mapping stored in tag_mappings.
/// </summary>
public sealed class TagMapping
{
    /// <summary>
    /// Username key in Rust schema.
    /// </summary>
    public required string Username { get; init; }

    /// <summary>
    /// 8-character tag.
    /// </summary>
    public required string Tag { get; init; }

    /// <summary>
    /// Unix seconds when mapping was created.
    /// </summary>
    public long CreatedAtUnix { get; init; }

    /// <summary>
    /// Convenience view for existing XAML bindings.
    /// </summary>
    public string Identity => Username;

    /// <summary>
    /// Rust schema does not have display_name.
    /// </summary>
    public string? DisplayName => null;

    /// <summary>
    /// Keeps compatibility with existing "UpdatedAt" display binding.
    /// </summary>
    public DateTime UpdatedAt => DateTimeOffset.FromUnixTimeSeconds(CreatedAtUnix).UtcDateTime;

    public DateTime CreatedAt => UpdatedAt;

    /// <summary>
    /// UI-only selected state for delete mode.
    /// </summary>
    public bool IsSelected { get; set; }
}
