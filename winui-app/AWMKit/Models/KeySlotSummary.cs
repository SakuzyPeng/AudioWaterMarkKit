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

    public string SlotTitle => IsActive ? L($"槽位 {Slot}（激活）", $"Slot {Slot} (active)") : L($"槽位 {Slot}", $"Slot {Slot}");

    public string StatusDisplayText => StatusText switch
    {
        "active" => L("激活", "Active"),
        "configured" => L("已配置", "Configured"),
        "duplicate" => L("重复", "Duplicate"),
        _ => L("未配置", "Empty")
    };

    public string TitleWithStatus => $"{SlotTitle} · {StatusDisplayText}";

    public string KeyLine
    {
        get
        {
            if (!HasKey)
            {
                return L("未配置", "Not configured");
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
            var baseText = L($"证据: {EvidenceCount}", $"Evidence: {EvidenceCount}");
            if (DuplicateOfSlots.Length == 0)
            {
                return baseText;
            }

            return L($"{baseText} · 重复: {string.Join(",", DuplicateOfSlots)}", $"{baseText} · duplicate: {string.Join(",", DuplicateOfSlots)}");
        }
    }

    private static string L(string zh, string en) => AppViewModel.Instance.IsEnglishLanguage ? en : zh;
}
