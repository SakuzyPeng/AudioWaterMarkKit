using System;
using AWMKit.ViewModels;

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

    public string SlotTitle => IsActive ? AppStrings.Pick($"槽位 {Slot}（激活）", $"Slot {Slot} (active)") : AppStrings.Pick($"槽位 {Slot}", $"Slot {Slot}");

    public string StatusDisplayText => StatusText switch
    {
        "active" => AppStrings.Pick("激活", "Active"),
        "configured" => AppStrings.Pick("已配置", "Configured"),
        "duplicate" => AppStrings.Pick("重复", "Duplicate"),
        _ => AppStrings.Pick("未配置", "Empty")
    };

    public string TitleWithStatus => $"{SlotTitle} · {StatusDisplayText}";

    public string KeyLine
    {
        get
        {
            if (!HasKey)
            {
                return AppStrings.Pick("未配置", "Not configured");
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
            var baseText = AppStrings.Pick($"证据: {EvidenceCount}", $"Evidence: {EvidenceCount}");
            if (DuplicateOfSlots.Length == 0)
            {
                return baseText;
            }

            return AppStrings.Pick($"{baseText} · 重复: {string.Join(",", DuplicateOfSlots)}", $"{baseText} · duplicate: {string.Join(",", DuplicateOfSlots)}");
        }
    }


}
