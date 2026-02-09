using AWMKit.Native;

namespace AWMKit.Models;

/// <summary>
/// User-facing channel layout option for multichannel embed/detect.
/// </summary>
public sealed record ChannelLayoutOption(AwmChannelLayout Layout, string Label, uint Channels)
{
    public string DisplayText => Channels == 0 ? Label : $"{Label} ({Channels}ch)";
}
