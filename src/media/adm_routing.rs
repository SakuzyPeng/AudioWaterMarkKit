//! ADM Bed 声道路由：基于 axml speakerLabel 构建 RoutePlan
//!
//! 支持两种标签体系：
//! - ITU-R BS.2076 标准标签（`M+030`, `M-030`, `M+000`, `LFE1` …）
//! - Dolby Room-Centric 标签（`RC_L`, `RC_R`, `RC_C`, `RC_LFE` …）
//! - 简单 L/R/C/LFE 标签
//!
//! 链路：
//! ```text
//! chna: trackIndex → AT_xxxxxxxx (audioTrackFormatIDRef)
//! axml: AT_xxxxxxxx_01 → AS_xxxxxxxx → AC_xxxxxxxx → speakerLabel
//! ```

use std::collections::HashMap;

use quick_xml::Reader;
use quick_xml::events::Event;

use crate::multichannel::{LfeMode, RouteMode, RoutePlan, RouteStep, SampleFormat};

// ─────────────────── 喇叭标签配对表 ───────────────────

/// 已知立体声对：(left_label, right_label, route_step_name)
///
/// 按优先级排序：低序号先匹配。
const SPEAKER_PAIRS: &[(&str, &str, &str)] = &[
    // ── ITU-R BS.2076 标准 ──
    ("M+030", "M-030", "FL+FR"),
    ("M+060", "M-060", "FLM+FRM"),
    ("M+090", "M-090", "SL+SR"),
    ("M+110", "M-110", "BL+BR"),
    ("M+135", "M-135", "BL+BR"),
    ("U+030", "U-030", "TFL+TFR"),
    ("U+045", "U-045", "TFL+TFR"),
    ("U+060", "U-060", "TFLs+TFRs"),
    ("U+090", "U-090", "TSL+TSR"),
    ("U+110", "U-110", "TBL+TBR"),
    ("U+135", "U-135", "TBL+TBR"),
    ("B+030", "B-030", "BFL+BFR"),
    ("B+045", "B-045", "BFL+BFR"),
    // ── Dolby Room-Centric (RC_) ──
    ("RC_L",   "RC_R",   "FL+FR"),
    ("RC_Ls",  "RC_Rs",  "SL+SR"),
    ("RC_Lss", "RC_Rss", "SL+SR"),
    ("RC_Lrs", "RC_Rrs", "BL+BR"),
    ("RC_Lts", "RC_Rts", "TFL+TFR"),
    ("RC_Lhs", "RC_Rhs", "TSL+TSR"),
    ("RC_Lbs", "RC_Rbs", "TBL+TBR"),
    ("RC_Lvs", "RC_Rvs", "TML+TMR"),
    // ── 简单标签 ──
    ("L",   "R",   "FL+FR"),
    ("Ls",  "Rs",  "SL+SR"),
    ("Lss", "Rss", "SL+SR"),
    ("Lrs", "Rrs", "BL+BR"),
    ("Lts", "Rts", "TFL+TFR"),
];

/// Centre 类单声道标签
const CENTRE_LABELS: &[&str] = &["M+000", "U+000", "T+000", "RC_C", "C"];

/// LFE 标签
const LFE_LABELS: &[&str] = &["LFE1", "LFE2", "LFE", "LFE+000", "RC_LFE"];

// ─────────────────── axml 映射解析 ───────────────────

/// ADM XML 三张映射表
///
/// - `track_to_stream`: `AT_xxxxxxxx` → `AS_xxxxxxxx`
/// - `stream_to_chan`:  `AS_xxxxxxxx` → `AC_xxxxxxxx`
/// - `chan_to_label`:   `AC_xxxxxxxx` → speakerLabel
#[derive(Debug, Default)]
pub struct AdmMaps {
    track_to_stream: HashMap<String, String>,
    stream_to_chan: HashMap<String, String>,
    chan_to_label: HashMap<String, String>,
}

impl AdmMaps {
    /// 通过 `AT_xxxxxxxx` 解析到 speakerLabel。
    ///
    /// 解析链路：`AT` → `AS` → `AC` → label。
    pub(crate) fn resolve_at_to_label(&self, at_id: &str) -> Option<&str> {
        let stream_id = self.track_to_stream.get(at_id)?;
        let channel_id = self.stream_to_chan.get(stream_id)?;
        self.chan_to_label.get(channel_id).map(String::as_str)
    }
}

/// 从 axml 字节流（UTF-8 XML）解析三张 ADM 映射表。
///
/// 遇到解析错误时提前退出，已解析部分仍可用。
pub fn parse_adm_maps(xml_bytes: &[u8]) -> AdmMaps {
    let mut maps = AdmMaps::default();

    let mut reader = Reader::from_reader(xml_bytes);
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();

    let mut cur_track_id: Option<String> = None;
    let mut cur_stream_id: Option<String> = None;
    let mut cur_chan_id: Option<String> = None;
    let mut in_stream_fmt_ref = false;
    let mut in_chan_fmt_ref = false;
    let mut in_speaker_label = false;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e) | Event::Empty(ref e)) => {
                let name_b = e.name();
                let local_b = name_b.local_name();
                let local = std::str::from_utf8(local_b.as_ref()).unwrap_or("");

                match local {
                    "audioTrackFormat" => {
                        cur_track_id = get_attr(e, "audioTrackFormatID")
                            .map(|id| strip_track_fmt_suffix(&id));
                    }
                    "audioStreamFormat" => {
                        cur_stream_id = get_attr(e, "audioStreamFormatID");
                    }
                    "audioChannelFormat" => {
                        cur_chan_id = get_attr(e, "audioChannelFormatID");
                    }
                    "audioStreamFormatIDRef" if cur_track_id.is_some() => {
                        in_stream_fmt_ref = true;
                    }
                    "audioChannelFormatIDRef" if cur_stream_id.is_some() => {
                        in_chan_fmt_ref = true;
                    }
                    "speakerLabel" if cur_chan_id.is_some() => {
                        in_speaker_label = true;
                    }
                    _ => {}
                }
            }
            Ok(Event::End(ref e)) => {
                let name_b = e.name();
                let local_b = name_b.local_name();
                let local = std::str::from_utf8(local_b.as_ref()).unwrap_or("");
                match local {
                    "audioTrackFormat"       => { cur_track_id  = None; in_stream_fmt_ref = false; }
                    "audioStreamFormat"      => { cur_stream_id = None; in_chan_fmt_ref   = false; }
                    "audioChannelFormat"     => { cur_chan_id   = None; }
                    "audioStreamFormatIDRef" => { in_stream_fmt_ref = false; }
                    "audioChannelFormatIDRef"=> { in_chan_fmt_ref   = false; }
                    "speakerLabel"           => { in_speaker_label  = false; }
                    _ => {}
                }
            }
            Ok(Event::Text(ref t)) => {
                let text = match std::str::from_utf8(t.as_ref()) {
                    Ok(s) => s.trim().to_string(),
                    Err(_) => continue,
                };
                if text.is_empty() {
                    // nothing
                } else if in_stream_fmt_ref {
                    if let Some(ref at_id) = cur_track_id {
                        maps.track_to_stream.insert(at_id.clone(), text);
                    }
                    in_stream_fmt_ref = false;
                } else if in_chan_fmt_ref {
                    if let Some(ref as_id) = cur_stream_id {
                        maps.stream_to_chan.insert(as_id.clone(), text);
                    }
                    in_chan_fmt_ref = false;
                } else if in_speaker_label {
                    // 只记每个 AC_ 的第一个 speakerLabel
                    if let Some(ref ac_id) = cur_chan_id {
                        maps.chan_to_label.entry(ac_id.clone()).or_insert(text);
                    }
                    in_speaker_label = false;
                }
            }
            Ok(Event::Eof) | Err(_) => break,
            _ => {}
        }
        buf.clear();
    }

    maps
}

/// 从 XML 元素中提取属性值。
fn get_attr(e: &quick_xml::events::BytesStart<'_>, attr_name: &str) -> Option<String> {
    e.attributes()
        .filter_map(std::result::Result::ok)
        .find(|a| {
            let key_b = a.key.local_name();
            std::str::from_utf8(key_b.as_ref()).unwrap_or("") == attr_name
        })
        .and_then(|a| std::str::from_utf8(a.value.as_ref()).ok().map(str::to_string))
}

/// `AT_00011001_01` → `AT_00011001`（去掉末尾的 `_XX` index 后缀）。
fn strip_track_fmt_suffix(id: &str) -> String {
    if let Some(pos) = id.rfind('_') {
        let suffix = &id[pos + 1..];
        let base = &id[..pos];
        if !suffix.is_empty()
            && suffix.chars().all(|c| c.is_ascii_hexdigit())
            && base.starts_with("AT_")
            && base.len() > 3
        {
            return base.to_string();
        }
    }
    id.to_string()
}

// ─────────────────── RoutePlan 构建 ───────────────────

/// 基于 speakerLabel 构建 [`RoutePlan`]。
///
/// `channel_labels`: `(channelIndex, speakerLabel)` 列表，**仅包含 Bed 声道**。
///
/// `lfe_mode` 控制 LFE 声道处理方式（Skip / Mono / Pair）。
pub fn build_route_plan_from_labels(
    channel_labels: &[(usize, String)],
    lfe_mode: LfeMode,
) -> RoutePlan {
    let mut steps: Vec<RouteStep> = Vec::new();
    let mut used = vec![false; channel_labels.len()];
    let mut warnings: Vec<String> = Vec::new();

    // ── 1. 立体声对（按 SPEAKER_PAIRS 优先顺序）──
    for &(left_lbl, right_lbl, pair_name) in SPEAKER_PAIRS {
        let left = channel_labels
            .iter()
            .enumerate()
            .find(|(i, (_, l))| !used[*i] && l.as_str() == left_lbl);
        let right = channel_labels
            .iter()
            .enumerate()
            .find(|(i, (_, l))| !used[*i] && l.as_str() == right_lbl);
        if let (Some((li, (lch, _))), Some((ri, (rch, _)))) = (left, right) {
            steps.push(make_pair(*lch, *rch, pair_name));
            used[li] = true;
            used[ri] = true;
        }
    }

    // ── 2. Centre → Mono（真单声道，非复制）──
    for (slot, (ch, label)) in channel_labels.iter().enumerate() {
        if used[slot] {
            continue;
        }
        if CENTRE_LABELS.contains(&label.as_str()) {
            steps.push(make_mono(*ch, &format!("{label}(mono)")));
            used[slot] = true;
        }
    }

    // ── 3. LFE（受 lfe_mode 控制）──
    // 先收集所有 LFE 声道位置
    let lfe_slots: Vec<(usize, usize)> = channel_labels
        .iter()
        .enumerate()
        .filter(|(i, (_, l))| !used[*i] && LFE_LABELS.contains(&l.as_str()))
        .map(|(slot, (ch, _))| (slot, *ch))
        .collect();

    match lfe_mode {
        LfeMode::Skip => {
            for (slot, ch) in &lfe_slots {
                steps.push(make_skip(*ch, "lfe_skipped"));
                used[*slot] = true;
            }
        }
        LfeMode::Mono => {
            for (slot, ch) in &lfe_slots {
                let label = &channel_labels[*slot].1;
                steps.push(make_mono(*ch, &format!("{label}(mono)")));
                used[*slot] = true;
            }
        }
        LfeMode::Pair => {
            // 只有两个 LFE 时才能配对；否则各自 mono
            if lfe_slots.len() == 2 {
                let (s0, ch0) = lfe_slots[0];
                let (s1, ch1) = lfe_slots[1];
                let l0 = &channel_labels[s0].1;
                let l1 = &channel_labels[s1].1;
                steps.push(make_pair(ch0, ch1, &format!("{l0}+{l1}")));
                used[s0] = true;
                used[s1] = true;
            } else {
                for (slot, ch) in &lfe_slots {
                    let label = &channel_labels[*slot].1;
                    steps.push(make_mono(*ch, &format!("{label}(mono)")));
                    used[*slot] = true;
                }
            }
        }
    }

    // ── 4. 剩余未知标签 → 顺序两两配对，并发出警告 ──
    let remaining: Vec<(usize, usize, &String)> = channel_labels
        .iter()
        .enumerate()
        .filter(|(i, _)| !used[*i])
        .map(|(slot, (ch, l))| (slot, *ch, l))
        .collect();

    if !remaining.is_empty() {
        let unknown_labels: Vec<&str> = remaining.iter().map(|(_, _, l)| l.as_str()).collect();
        warnings.push(format!(
            "ADM: unknown speaker label(s) {unknown_labels:?}; \
             falling back to sequential pairing for these channels"
        ));
    }

    for pair in remaining.chunks(2) {
        match pair {
            [(_, ch0, l0), (_, ch1, l1)] => {
                steps.push(make_pair(*ch0, *ch1, &format!("{l0}+{l1}(unknown)")));
            }
            [(_, ch0, l0)] => {
                steps.push(make_mono(*ch0, &format!("{l0}(mono/unknown)")));
            }
            _ => {}
        }
    }

    // channels / layout 对 RoutePlan 是必填字段，用实际值填充
    let channels = channel_labels.len();
    let layout = crate::multichannel::ChannelLayout::from_channels(
        u16::try_from(channels).unwrap_or(u16::MAX),
    );

    RoutePlan { layout, channels, steps, warnings }
}

// ─────────────────── 静默检测 ─────────────────────────

/// 静默检测：声道峰值低于约 -80 dBFS 时返回 `true`。
///
/// ADM Object 声道嵌入/检测前调用，避免对静默声道浪费 audiowmark 调用。
///
/// 阈值换算（`max_val / 10_000` ≈ -80 dBFS）：
/// - `Int16`  : 32_767 / 10_000 = 3
/// - `Int24`  : 8_388_607 / 10_000 = 838
/// - `Int32` / `Float32` : i32::MAX / 10_000 = 214_748
///
/// 注意：`Float32` 样本在 [`MultichannelAudio`] 中同样以 i32 存储，
/// 归一化值 [-1.0, 1.0] 转换后绝对值远低于 i32::MAX，因此阈值仍能
/// 有效过滤真正静默的 Float32 声道。
#[must_use]
pub fn is_silent(samples: &[i32], format: SampleFormat) -> bool {
    let max_val: i32 = match format {
        SampleFormat::Int16 => 32_767,
        SampleFormat::Int24 => 8_388_607,
        SampleFormat::Int32 | SampleFormat::Float32 => i32::MAX,
    };
    let threshold = (max_val / 10_000).max(1);
    samples.iter().all(|&s| s.abs() < threshold)
}

// ─── 辅助构造 ─────────────────────────────────────────

fn make_pair(ch_a: usize, ch_b: usize, name: &str) -> RouteStep {
    RouteStep { name: name.to_string(), mode: RouteMode::Pair(ch_a, ch_b) }
}

fn make_mono(channel: usize, name: &str) -> RouteStep {
    RouteStep { name: name.to_string(), mode: RouteMode::Mono(channel) }
}

fn make_skip(channel: usize, reason: &'static str) -> RouteStep {
    RouteStep { name: format!("ch{channel}(skip)"), mode: RouteMode::Skip { channel, reason } }
}

// ─────────────────── 测试 ───────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::multichannel::{DEFAULT_LFE_MODE, LfeMode};

    fn label_vec(labels: &[&str]) -> Vec<(usize, String)> {
        labels
            .iter()
            .enumerate()
            .map(|(i, &l)| (i, l.to_string()))
            .collect()
    }

    #[test]
    fn atmos_7_1_4_bs2076() {
        // 7.1.4: M+030 M-030 M+000 LFE1 M+110 M-110 M+090 M-090 U+030 U-030 U+110 U-110
        let labels = label_vec(&[
            "M+030", "M-030", "M+000", "LFE1",
            "M+110", "M-110", "M+090", "M-090",
            "U+030", "U-030", "U+110", "U-110",
        ]);
        let plan = build_route_plan_from_labels(&labels, DEFAULT_LFE_MODE);
        assert!(plan.warnings.is_empty(), "no unknown labels expected: {:?}", plan.warnings);

        let modes: Vec<&RouteMode> = plan.steps.iter().map(|s| &s.mode).collect();
        // FL+FR
        assert!(matches!(modes[0], RouteMode::Pair(0, 1)));
        // Centre mono
        assert!(matches!(modes.iter().find(|m| matches!(m, RouteMode::Mono(2))), Some(_)));
        // LFE skip
        assert!(matches!(modes.iter().find(|m| matches!(m, RouteMode::Skip { channel: 3, .. })), Some(_)));
    }

    #[test]
    fn dolby_rc_labels_7_1_4() {
        // 7.1.4 RC_ 标签（和 ADM BWF 测试文件相同格式）
        let labels = label_vec(&[
            "RC_L", "RC_R", "RC_C", "RC_LFE",
            "RC_Lss", "RC_Rss", "RC_Lrs", "RC_Rrs",
            "RC_Lts", "RC_Rts",
        ]);
        let plan = build_route_plan_from_labels(&labels, DEFAULT_LFE_MODE);
        assert!(plan.warnings.is_empty(), "unexpected warnings: {:?}", plan.warnings);

        let names: Vec<&str> = plan.steps.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"FL+FR"), "FL+FR missing");
        assert!(names.contains(&"SL+SR"), "SL+SR missing");
        assert!(names.contains(&"BL+BR"), "BL+BR missing");
        assert!(names.contains(&"TFL+TFR"), "TFL+TFR missing");
        assert!(names.iter().any(|n| n.contains("mono")), "Centre mono missing");
        assert!(names.iter().any(|n| n.contains("skip")), "LFE skip missing");
    }

    #[test]
    fn lfe_mono_mode() {
        let labels = label_vec(&["M+030", "M-030", "M+000", "LFE1", "M+110", "M-110"]);
        let plan = build_route_plan_from_labels(&labels, LfeMode::Mono);
        let lfe_step = plan.steps.iter().find(|s| s.name.contains("LFE1"));
        assert!(lfe_step.is_some());
        assert!(matches!(lfe_step.unwrap().mode, RouteMode::Mono(3)));
    }

    #[test]
    fn lfe_pair_mode_two_lfe() {
        let labels = label_vec(&["M+030", "M-030", "LFE1", "LFE2"]);
        let plan = build_route_plan_from_labels(&labels, LfeMode::Pair);
        let pair = plan.steps.iter().find(|s| matches!(s.mode, RouteMode::Pair(2, 3)));
        assert!(pair.is_some(), "LFE1+LFE2 pair expected");
    }

    #[test]
    fn unknown_labels_emit_warning() {
        let labels = label_vec(&["M+030", "M-030", "FooLeft", "FooRight"]);
        let plan = build_route_plan_from_labels(&labels, DEFAULT_LFE_MODE);
        assert!(!plan.warnings.is_empty(), "warning expected for unknown labels");
    }

    #[test]
    fn parse_adm_maps_chain() {
        // 最小 axml：一个 AT→AS→AC→speakerLabel 链路
        let xml = r#"<?xml version="1.0"?>
<audioFormatExtended>
  <audioTrackFormat audioTrackFormatID="AT_00011001_01">
    <audioStreamFormatIDRef>AS_00011001</audioStreamFormatIDRef>
  </audioTrackFormat>
  <audioStreamFormat audioStreamFormatID="AS_00011001">
    <audioChannelFormatIDRef>AC_00011001</audioChannelFormatIDRef>
  </audioStreamFormat>
  <audioChannelFormat audioChannelFormatID="AC_00011001">
    <audioBlockFormat><speakerLabel>M+030</speakerLabel></audioBlockFormat>
  </audioChannelFormat>
</audioFormatExtended>"#;
        let maps = parse_adm_maps(xml.as_bytes());
        assert_eq!(maps.resolve_at_to_label("AT_00011001"), Some("M+030"));
    }

    #[test]
    fn strip_suffix_basic() {
        assert_eq!(strip_track_fmt_suffix("AT_00011001_01"), "AT_00011001");
        assert_eq!(strip_track_fmt_suffix("AT_00011001"),    "AT_00011001");
        assert_eq!(strip_track_fmt_suffix("AT_0001100a_01"), "AT_0001100a");
    }

    #[test]
    fn is_silent_int16_true_on_zero() {
        use crate::multichannel::SampleFormat;
        let samples = vec![0_i32; 100];
        assert!(is_silent(&samples, SampleFormat::Int16));
    }

    #[test]
    fn is_silent_int16_false_on_loud() {
        use crate::multichannel::SampleFormat;
        // 1000 > 32767/10000=3 threshold
        let samples = vec![1_000_i32; 100];
        assert!(!is_silent(&samples, SampleFormat::Int16));
    }

    #[test]
    fn is_silent_int24_threshold() {
        use crate::multichannel::SampleFormat;
        let threshold = 8_388_607_i32 / 10_000; // 838
        // 样本峰值 = threshold - 1 → 静默
        let quiet = vec![threshold - 1; 50];
        assert!(is_silent(&quiet, SampleFormat::Int24));
        // 样本峰值 = threshold → 非静默
        let loud = vec![threshold; 50];
        assert!(!is_silent(&loud, SampleFormat::Int24));
    }

    #[test]
    fn is_silent_int32_threshold() {
        use crate::multichannel::SampleFormat;
        let threshold = i32::MAX / 10_000; // 214_748
        let quiet = vec![threshold - 1; 10];
        assert!(is_silent(&quiet, SampleFormat::Int32));
        let loud = vec![threshold; 10];
        assert!(!is_silent(&loud, SampleFormat::Int32));
    }

    #[test]
    fn is_silent_empty_slice() {
        use crate::multichannel::SampleFormat;
        // 空切片：所有迭代器上的 all() 对空集合返回 true
        assert!(is_silent(&[], SampleFormat::Int24));
    }
}
