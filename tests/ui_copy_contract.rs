use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

fn parse_resw_pairs(path: &Path) -> BTreeMap<String, String> {
    let content = fs::read_to_string(path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()));

    let mut pairs = BTreeMap::new();
    for line in content.lines() {
        let line = line.trim();
        if !line.starts_with("<data name=\"") {
            continue;
        }

        let start = "<data name=\"".len();
        let Some(end) = line[start..].find('"') else {
            continue;
        };
        let key = &line[start..start + end];

        let Some(value_start) = line.find("<value>") else {
            continue;
        };
        let Some(value_end) = line.find("</value>") else {
            continue;
        };

        let value = line[value_start + "<value>".len()..value_end].trim();
        pairs.insert(key.to_string(), value.to_string());
    }

    pairs
}

#[test]
fn next_step_error_copy_exists_in_both_languages() {
    let en = parse_resw_pairs(Path::new("winui-app/AWMKit/Strings/en-US/Resources.resw"));
    let zh = parse_resw_pairs(Path::new("winui-app/AWMKit/Strings/zh-CN/Resources.resw"));

    let required = [
        "ui.error.next.open_diagnostics",
        "ui.error.next.check_input_path",
        "ui.error.next.retry",
    ];

    for key in required {
        let en_value = en.get(key).unwrap_or_else(|| panic!("missing en key: {key}"));
        let zh_value = zh.get(key).unwrap_or_else(|| panic!("missing zh key: {key}"));
        assert!(
            en_value.contains("Next:"),
            "en value for {key} must contain Next:"
        );
        assert!(
            zh_value.contains("下一步："),
            "zh value for {key} must contain 下一步："
        );
    }
}

#[test]
fn default_ui_copy_hides_internal_fields() {
    let en = parse_resw_pairs(Path::new("winui-app/AWMKit/Strings/en-US/Resources.resw"));
    let zh = parse_resw_pairs(Path::new("winui-app/AWMKit/Strings/zh-CN/Resources.resw"));
    let banned = ["route=", "status=", "single_fallback", "UNVERIFIED"];

    for (key, value) in en.iter().chain(zh.iter()) {
        if key.starts_with("ui.error.") || key.starts_with("ui.status.") || key.starts_with("ui.nav.") {
            for token in banned {
                assert!(
                    !value.contains(token),
                    "default ui copy key {key} must not expose internal field token {token}"
                );
            }
        }
    }
}

#[test]
fn glossary_terms_are_consistent_in_navigation() {
    let en = parse_resw_pairs(Path::new("winui-app/AWMKit/Strings/en-US/Resources.resw"));
    let zh = parse_resw_pairs(Path::new("winui-app/AWMKit/Strings/zh-CN/Resources.resw"));

    assert_eq!(en.get("ui.nav.tags").map(String::as_str), Some("Tags"));
    assert_eq!(zh.get("ui.nav.tags").map(String::as_str), Some("标签"));
    assert_eq!(en.get("ui.status.unavailable").map(String::as_str), Some("Unavailable"));
    assert_eq!(zh.get("ui.status.unavailable").map(String::as_str), Some("不可用"));
}
