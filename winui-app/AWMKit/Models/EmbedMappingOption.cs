namespace AWMKit.Models;

/// <summary>
/// Username -> tag mapping option shown in embed page.
/// </summary>
public sealed class EmbedMappingOption
{
    public required string Username { get; init; }
    public required string Tag { get; init; }

    public string DisplayText => $"{Username}（{Tag}）";
}
