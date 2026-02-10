using System;

namespace AWMKit.Models;

/// <summary>
/// Key slot summary projected from Rust KeyStore.
/// </summary>
public sealed class KeySlotSummary
{
    public int Slot { get; init; }
    public bool IsActive { get; init; }
    public bool HasKey { get; init; }
    public string? KeyId { get; init; }
    public string? Label { get; init; }
    public int EvidenceCount { get; init; }
    public long? LastEvidenceAt { get; init; }
    public string StatusText { get; init; } = "empty";
    public int[] DuplicateOfSlots { get; init; } = Array.Empty<int>();

    public string SlotTitle => IsActive ? $"槽位 {Slot}（激活）" : $"槽位 {Slot}";

    public string StatusDisplayText => StatusText switch
    {
        "active" => "激活",
        "configured" => "已配置",
        "duplicate" => "重复",
        _ => "未配置"
    };

    public string TitleWithStatus => $"{SlotTitle} · {StatusDisplayText}";

    public string KeyLine
    {
        get
        {
            if (!HasKey)
            {
                return "未配置";
            }

            if (string.IsNullOrWhiteSpace(Label))
            {
                return $"Key ID: {KeyId ?? "-"}";
            }

            return $"Key ID: {KeyId ?? "-"} · {Label}";
        }
    }

    public string EvidenceLine
    {
        get
        {
            var baseText = $"证据: {EvidenceCount}";
            if (DuplicateOfSlots.Length == 0)
            {
                return baseText;
            }

            return $"{baseText} · 重复: {string.Join(",", DuplicateOfSlots)}";
        }
    }
}
