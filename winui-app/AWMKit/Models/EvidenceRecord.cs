using System;
using AWMKit.ViewModels;

namespace AWMKit.Models;

/// <summary>
/// Represents one row from audio_evidence.
/// </summary>
public sealed class EvidenceRecord
{
    public long Id { get; init; }
    public long CreatedAt { get; init; }
    public required string FilePath { get; init; }
    public required string Tag { get; init; }
    public required string Identity { get; init; }
    public int Version { get; init; }
    public int KeySlot { get; init; }
    public long TimestampMinutes { get; init; }
    public required string MessageHex { get; init; }
    public int SampleRate { get; init; }
    public int Channels { get; init; }
    public long SampleCount { get; init; }
    public required string PcmSha256 { get; init; }
    public string? KeyId { get; init; }
    public double? SnrDb { get; init; }
    public string SnrStatus { get; init; } = "unavailable";
    public required byte[] ChromaprintBlob { get; init; }
    public int FingerprintLen { get; init; }
    public int FpConfigId { get; init; }

    public DateTime CreatedAtDateTime => DateTimeOffset.FromUnixTimeSeconds(CreatedAt).UtcDateTime;

    // Compatibility shims for existing WinUI code paths that still reference old names.
    public string FileHash => PcmSha256;
    public string Message => MessageHex;
    public string Pattern => "-";
    public string TagDisplayText => $"Tag {Tag}";
    public string KeyIdDisplayText => string.IsNullOrWhiteSpace(KeyId) ? "-" : KeyId;
    public string TagSlotDisplayText
    {
        get
        {
            var snr = string.Equals(SnrStatus, "ok", StringComparison.OrdinalIgnoreCase) && SnrDb.HasValue
                ? $" · SNR {SnrDb.Value:F2} dB"
                : string.Empty;
            return L(
                $"Tag {Tag} · 槽位 {KeySlot} · Key ID {KeyIdDisplayText}{snr}",
                $"Tag {Tag} · Slot {KeySlot} · Key ID {KeyIdDisplayText}{snr}");
        }
    }

    /// <summary>
    /// UI-only selected state for delete mode.
    /// </summary>
    public bool IsSelected { get; set; }

    private static string L(string zh, string en) => AppViewModel.Instance.IsEnglishLanguage ? en : zh;
}
