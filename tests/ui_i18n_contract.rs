use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

fn parse_resw_ui_keys(path: &Path) -> BTreeSet<String> {
    let content = fs::read_to_string(path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()));

    content
        .lines()
        .filter_map(|line| {
            let start = line.find("<data name=\"")? + "<data name=\"".len();
            let rest = &line[start..];
            let end = rest.find('"')?;
            let key = &rest[..end];
            if key.starts_with("ui.") {
                Some(key.to_string())
            } else {
                None
            }
        })
        .collect()
}

fn parse_localizer_ui_fallback_keys(path: &Path) -> BTreeSet<String> {
    let content = fs::read_to_string(path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()));

    content
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim_start();
            if !trimmed.starts_with('"') {
                return None;
            }

            let end = trimmed[1..].find('"')? + 1;
            let key = &trimmed[1..end];
            if key.starts_with("ui.") {
                Some(key.to_string())
            } else {
                None
            }
        })
        .collect()
}

#[test]
fn winui_ui_keys_match_between_en_and_zh() {
    let en = parse_resw_ui_keys(Path::new("winui-app/AWMKit/Strings/en-US/Resources.resw"));
    let zh = parse_resw_ui_keys(Path::new("winui-app/AWMKit/Strings/zh-CN/Resources.resw"));
    assert_eq!(en, zh, "winui ui.* key sets must match between en-US and zh-CN");
}

#[test]
fn critical_ui_keys_exist() {
    let keys = parse_resw_ui_keys(Path::new("winui-app/AWMKit/Strings/en-US/Resources.resw"));
    let required = [
        "ui.nav.embed",
        "ui.nav.detect",
        "ui.nav.tags",
        "ui.nav.key",
        "ui.status.key",
        "ui.status.engine",
        "ui.status.database",
        "ui.toggle.show_diagnostics",
        "ui.error.processing_failed",
        "ui.error.next.open_diagnostics",
    ];

    for key in required {
        assert!(keys.contains(key), "required UI key missing: {key}");
    }
}

#[test]
fn macos_localizer_has_ui_fallback_keys() {
    let keys = parse_localizer_ui_fallback_keys(Path::new(
        "macos-app/AWMKit/Sources/Localization/Localizer.swift",
    ));
    assert!(keys.contains("ui.sidebar.appearance"));
    assert!(keys.contains("ui.sidebar.appearance.help"));
}

#[test]
fn disallow_local_bilingual_helper_definitions() {
    let mut offending = Vec::new();
    let roots = [
        Path::new("macos-app/AWMKit/Sources"),
        Path::new("winui-app/AWMKit"),
    ];

    for root in roots {
        let walker = fs::read_dir(root).unwrap_or_else(|err| panic!("read_dir {} failed: {err}", root.display()));
        collect_offending(root, walker, &mut offending);
    }

    assert!(
        offending.is_empty(),
        "found forbidden local bilingual helper definitions:\n{}",
        offending.join("\n")
    );
}

fn collect_offending(
    root: &Path,
    entries: std::fs::ReadDir,
    offending: &mut Vec<String>,
) {
    for entry in entries {
        let entry = entry.unwrap_or_else(|err| panic!("failed to read dir entry under {}: {err}", root.display()));
        let path = entry.path();
        if path.is_dir() {
            let child = fs::read_dir(&path)
                .unwrap_or_else(|err| panic!("read_dir {} failed: {err}", path.display()));
            collect_offending(&path, child, offending);
            continue;
        }

        let ext = path.extension().and_then(|v| v.to_str()).unwrap_or_default();
        if ext != "swift" && ext != "cs" {
            continue;
        }

        let content = fs::read_to_string(&path)
            .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()));
        for (lineno, line) in content.lines().enumerate() {
            let trimmed = line.trim();
            let is_forbidden = trimmed.contains("private func l(")
                || trimmed.contains("private func localized(")
                || trimmed.contains("private static string L(")
                || trimmed.contains("private string L(");
            if is_forbidden {
                offending.push(format!("{}:{}: {}", path.display(), lineno + 1, trimmed));
            }
        }
    }
}
