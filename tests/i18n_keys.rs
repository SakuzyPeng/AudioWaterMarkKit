use std::collections::BTreeSet;
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
