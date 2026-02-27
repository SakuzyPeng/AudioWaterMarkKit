use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

fn parse_ftl_keys(path: &Path) -> BTreeSet<String> {
    let content = fs::read_to_string(path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()));
    let mut keys = BTreeSet::new();
    for line in content.lines() {
        if let Some((key, _)) = line.split_once('=') {
            let trimmed = key.trim();
            if !trimmed.is_empty()
                && trimmed
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_')
            {
                keys.insert(trimmed.to_string());
            }
        }
    }
    keys
}

fn parse_ftl_pairs(path: &Path) -> BTreeMap<String, String> {
    let content = fs::read_to_string(path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()));
    let mut pairs = BTreeMap::new();
    for line in content.lines() {
        if let Some((key, value)) = line.split_once('=') {
            let key = key.trim();
            if !key.is_empty()
                && key
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_')
            {
                pairs.insert(key.to_string(), value.trim().to_string());
            }
        }
    }
    pairs
}

#[test]
fn i18n_key_sets_match_between_en_and_zh() {
    let en = parse_ftl_keys(Path::new("i18n/en-US/awmkit.ftl"));
    let zh = parse_ftl_keys(Path::new("i18n/zh-CN/awmkit.ftl"));
    assert_eq!(en, zh, "en-US and zh-CN i18n keys must match exactly");
}

#[test]
fn critical_cli_contract_keys_exist() {
    let keys = parse_ftl_keys(Path::new("i18n/en-US/awmkit.ftl"));
    let required = [
        "cli-error-database",
        "cli-error-config",
        "cli-error-io",
        "cli-error-audio",
        "cli-status-db-mappings",
        "cli-status-db-evidence",
        "cli-embed-file-ok",
        "cli-embed-file-failed",
        "cli-detect-file-found",
        "cli-detect-file-invalid",
        "cli-evidence-list-row",
        "cli-key-slot-list-row",
    ];
    for key in required {
        assert!(keys.contains(key), "required i18n key missing: {key}");
    }
}

#[test]
fn zh_detect_labels_are_localized() {
    let zh =
        fs::read_to_string("i18n/zh-CN/awmkit.ftl").expect("failed to read zh-CN localization");
    assert!(zh.contains("ui-detect-detail-tag = 标签"));
    assert!(zh.contains("ui-detect-detail-pattern = 检测模式"));
    assert!(zh.contains("ui-detect-detail-bit_errors = 比特错误数"));
    assert!(zh.contains("ui-detect-detail-match_found = 是否匹配"));
}

#[test]
fn critical_cli_errors_include_next_step_hints() {
    let en = parse_ftl_pairs(Path::new("i18n/en-US/awmkit.ftl"));
    let zh = parse_ftl_pairs(Path::new("i18n/zh-CN/awmkit.ftl"));

    let required = [
        "cli-error-quiet_verbose_conflict",
        "cli-error-key_not_found",
        "cli-error-input_not_found",
        "cli-key-delete-requires-yes",
        "cli-evidence-clear-refuse-all",
        "cli-detect-file-invalid",
    ];

    for key in required {
        let en_value = en.get(key).unwrap_or_else(|| panic!("missing en key: {key}"));
        let zh_value = zh.get(key).unwrap_or_else(|| panic!("missing zh key: {key}"));
        assert!(
            en_value.contains("Next:"),
            "en copy should include next-step hint for {key}"
        );
        assert!(
            zh_value.contains("下一步："),
            "zh copy should include next-step hint for {key}"
        );
    }
}

#[test]
fn default_cli_copy_hides_internal_diagnostic_fields() {
    let en = parse_ftl_pairs(Path::new("i18n/en-US/awmkit.ftl"));
    let zh = parse_ftl_pairs(Path::new("i18n/zh-CN/awmkit.ftl"));
    let banned = [
        "slot_hint",
        "decode_slot_hint",
        "decode_slot_used",
        "slot_scan_count",
        "scan_count",
    ];

    for (key, value) in en.iter().chain(zh.iter()) {
        if key.starts_with("cli-") && !key.ends_with("-detail") {
            for token in banned {
                assert!(
                    !value.contains(token),
                    "default copy key {key} must not expose internal field {token}"
                );
            }
        }
    }
}

#[test]
fn glossary_terms_match_between_en_and_zh_samples() {
    let en = parse_ftl_pairs(Path::new("i18n/en-US/awmkit.ftl"));
    let zh = parse_ftl_pairs(Path::new("i18n/zh-CN/awmkit.ftl"));
    let samples = [
        ("cli-detect-file-found", "watermark", "水印"),
        ("cli-embed-file-ok", "watermark", "水印"),
        ("cli-key-error-invalid-slot-range", "slot", "槽位"),
        ("cli-evidence-empty", "evidence", "证据"),
        ("cli-status-db-mappings", "mapping", "映射"),
        ("cli-status-value-unavailable", "unavailable", "不可用"),
    ];

    for (key, en_term, zh_term) in samples {
        let en_value = en.get(key).unwrap_or_else(|| panic!("missing en key: {key}"));
        let zh_value = zh.get(key).unwrap_or_else(|| panic!("missing zh key: {key}"));
        assert!(
            en_value.to_ascii_lowercase().contains(en_term),
            "en key {key} should contain glossary term {en_term}"
        );
        assert!(
            zh_value.contains(zh_term),
            "zh key {key} should contain glossary term {zh_term}"
        );
    }
}
