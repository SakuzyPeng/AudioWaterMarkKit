//! ADM/BWF (RIFF/RF64/BW64) 探测与 chunk 索引

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

use bwavfile::WaveReader;

use crate::error::{Error, Result};

const RIFF_SIG: [u8; 4] = *b"RIFF";
const RF64_SIG: [u8; 4] = *b"RF64";
const BW64_SIG: [u8; 4] = *b"BW64";
const WAVE_SIG: [u8; 4] = *b"WAVE";
const DS64_SIG: [u8; 4] = *b"ds64";
const FMT_SIG: [u8; 4] = *b"fmt ";
const DATA_SIG: [u8; 4] = *b"data";
const AXML_SIG: [u8; 4] = *b"axml";
const CHNA_SIG: [u8; 4] = *b"chna";
const BEXT_SIG: [u8; 4] = *b"bext";
const IXML_SIG: [u8; 4] = *b"iXML";
const U32_MAX_U64: u64 = 0xFFFF_FFFF;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WaveContainer {
    Riff,
    Rf64,
    Bw64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ChunkEntry {
    pub id: [u8; 4],
    pub header_offset: u64,
    pub data_offset: u64,
    pub size: u64,
    pub padded_size: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PcmFormat {
    pub channels: u16,
    pub sample_rate: u32,
    pub bits_per_sample: u16,
    pub block_align: u16,
    pub bytes_per_sample: u16,
}

#[derive(Debug, Clone)]
pub(crate) struct ChunkIndex {
    pub container: WaveContainer,
    pub file_size: u64,
    pub chunks: Vec<ChunkEntry>,
    pub fmt: PcmFormat,
    pub data_chunk: ChunkEntry,
    pub has_bext: bool,
    pub has_axml: bool,
    pub has_chna: bool,
    pub has_ixml: bool,
}

impl ChunkIndex {
    /// 文件是否需要 ADM 专用路径处理。
    ///
    /// 仅有 `bext` 的普通 BWF 文件不需要此路径——audiowmark 可以直接
    /// 处理其 PCM 数据，而无需精确保留 ADM 对象元数据。
    /// 真正的 ADM 文件需要有 `axml`（对象路由 XML）或 `chna`（声道编号）chunk。
    #[must_use]
    pub const fn is_adm_or_bwf(&self) -> bool {
        self.has_axml || self.has_chna
    }
}

pub(crate) fn probe_adm_bwf(path: &Path) -> Result<Option<ChunkIndex>> {
    let maybe_index = parse_chunk_index(path)?;
    let Some(index) = maybe_index else {
        return Ok(None);
    };

    if !index.is_adm_or_bwf() {
        return Ok(None);
    }

    validate_with_bwavfile(path)?;
    Ok(Some(index))
}

pub(crate) fn parse_chunk_index(path: &Path) -> Result<Option<ChunkIndex>> {
    let mut file = File::open(path)
        .map_err(|e| Error::AdmUnsupported(format!("failed to open {}: {e}", path.display())))?;
    let file_size = file
        .metadata()
        .map_err(|e| Error::AdmUnsupported(format!("failed to stat {}: {e}", path.display())))?
        .len();
    if file_size < 12 {
        return Ok(None);
    }

    let mut header = [0_u8; 12];
    file.read_exact(&mut header)
        .map_err(|e| Error::AdmUnsupported(format!("failed to read RIFF header: {e}")))?;
    let container_sig = [header[0], header[1], header[2], header[3]];
    let form_type = [header[8], header[9], header[10], header[11]];

    let container = match container_sig {
        RIFF_SIG => WaveContainer::Riff,
        RF64_SIG => WaveContainer::Rf64,
        BW64_SIG => WaveContainer::Bw64,
        _ => return Ok(None),
    };
    if form_type != WAVE_SIG {
        return Ok(None);
    }

    let mut parse_end = match container {
        WaveContainer::Riff => {
            let riff_size = u64::from(read_u32_le(&header[4..8])?);
            let end = checked_add(8, riff_size, "RIFF size overflow")?;
            end.min(file_size)
        }
        WaveContainer::Rf64 | WaveContainer::Bw64 => {
            if read_u32_le(&header[4..8])? != u32::MAX {
                return Err(Error::AdmUnsupported(
                    "RF64/BW64 header requires 0xFFFFFFFF size marker".to_string(),
                ));
            }
            file_size
        }
    };

    let mut cursor = 12_u64;
    let mut data_size_override: Option<u64> = None;

    if matches!(container, WaveContainer::Rf64 | WaveContainer::Bw64) {
        let (chunk_id, chunk_size) = read_chunk_header(&mut file, cursor)?;
        if chunk_id != DS64_SIG {
            return Err(Error::AdmUnsupported(
                "RF64/BW64 requires ds64 as first chunk".to_string(),
            ));
        }
        if chunk_size < 28 {
            return Err(Error::AdmUnsupported(
                "invalid ds64 chunk: payload shorter than 28 bytes".to_string(),
            ));
        }

        let mut ds64 = vec![
            0_u8;
            usize::try_from(chunk_size).map_err(|_| Error::AdmUnsupported(
                "ds64 chunk too large to load".to_string()
            ))?
        ];
        file.seek(SeekFrom::Start(cursor + 8))
            .map_err(|e| Error::AdmUnsupported(format!("failed to seek ds64 payload: {e}")))?;
        file.read_exact(&mut ds64)
            .map_err(|e| Error::AdmUnsupported(format!("failed to read ds64 payload: {e}")))?;

        let riff_size_64 = read_u64_le(&ds64[0..8])?;
        let data_size_64 = read_u64_le(&ds64[8..16])?;
        data_size_override = Some(data_size_64);

        let rf64_form_end = checked_add(8, riff_size_64, "RF64 form size overflow")?;
        parse_end = rf64_form_end.min(file_size);

        cursor = checked_add(cursor, 8, "ds64 cursor overflow")?;
        cursor = checked_add(cursor, u64::from(chunk_size), "ds64 cursor overflow")?;
        cursor = checked_add(cursor, u64::from(chunk_size & 1), "ds64 cursor overflow")?;
    }

    let mut chunks = Vec::new();
    while checked_add(cursor, 8, "chunk header overflow")? <= parse_end
        && checked_add(cursor, 8, "chunk header overflow")? <= file_size
    {
        let (chunk_id, chunk_size_field) = read_chunk_header(&mut file, cursor)?;
        let mut chunk_size = u64::from(chunk_size_field);
        if chunk_id == DATA_SIG && chunk_size == U32_MAX_U64 {
            let Some(override_size) = data_size_override else {
                return Err(Error::AdmUnsupported(
                    "data chunk uses RF64 size marker but ds64 has no size".to_string(),
                ));
            };
            chunk_size = override_size;
        }

        let data_offset = checked_add(cursor, 8, "chunk data offset overflow")?;
        let padded_size = checked_add(chunk_size, chunk_size & 1, "chunk pad overflow")?;
        let next = checked_add(data_offset, padded_size, "chunk next offset overflow")?;
        if next > file_size {
            return Err(Error::AdmUnsupported(format!(
                "chunk {} exceeds file size",
                fourcc_to_string(chunk_id)
            )));
        }

        chunks.push(ChunkEntry {
            id: chunk_id,
            header_offset: cursor,
            data_offset,
            size: chunk_size,
            padded_size,
        });
        cursor = next;
    }

    let fmt_chunk = chunks
        .iter()
        .find(|c| c.id == FMT_SIG)
        .copied()
        .ok_or_else(|| {
            Error::AdmUnsupported("missing fmt chunk in ADM/BWF candidate".to_string())
        })?;
    let data_chunk = chunks
        .iter()
        .find(|c| c.id == DATA_SIG)
        .copied()
        .ok_or_else(|| {
            Error::AdmUnsupported("missing data chunk in ADM/BWF candidate".to_string())
        })?;
    let fmt = read_pcm_format(&mut file, fmt_chunk)?;
    if fmt.block_align == 0 || data_chunk.size % u64::from(fmt.block_align) != 0 {
        return Err(Error::AdmUnsupported(format!(
            "data chunk is not aligned to block_align={} bytes",
            fmt.block_align
        )));
    }

    Ok(Some(ChunkIndex {
        container,
        file_size,
        chunks: chunks.clone(),
        fmt,
        data_chunk,
        has_bext: chunks.iter().any(|c| c.id == BEXT_SIG),
        has_axml: chunks.iter().any(|c| c.id == AXML_SIG),
        has_chna: chunks.iter().any(|c| c.id == CHNA_SIG),
        has_ixml: chunks.iter().any(|c| c.id == IXML_SIG),
    }))
}

fn validate_with_bwavfile(path: &Path) -> Result<()> {
    let mut reader = WaveReader::open(path).map_err(|e| {
        Error::AdmUnsupported(format!("bwavfile failed to open {}: {e}", path.display()))
    })?;
    reader
        .validate_readable()
        .map_err(|e| Error::AdmUnsupported(format!("bwavfile readable validation failed: {e}")))?;
    let mut axml = Vec::new();
    reader
        .read_axml(&mut axml)
        .map_err(|e| Error::AdmUnsupported(format!("bwavfile axml read failed: {e}")))?;
    Ok(())
}

fn read_pcm_format(file: &mut File, fmt_chunk: ChunkEntry) -> Result<PcmFormat> {
    if fmt_chunk.size < 16 {
        return Err(Error::AdmUnsupported(
            "invalid fmt chunk: payload shorter than 16 bytes".to_string(),
        ));
    }

    let payload_len = usize::try_from(fmt_chunk.size)
        .map_err(|_| Error::AdmUnsupported("fmt chunk too large to load".to_string()))?;
    let mut payload = vec![0_u8; payload_len];
    file.seek(SeekFrom::Start(fmt_chunk.data_offset))
        .map_err(|e| Error::AdmUnsupported(format!("failed to seek fmt chunk: {e}")))?;
    file.read_exact(&mut payload)
        .map_err(|e| Error::AdmUnsupported(format!("failed to read fmt chunk: {e}")))?;

    let format_tag = read_u16_le(&payload[0..2])?;
    let channels = read_u16_le(&payload[2..4])?;
    let sample_rate = read_u32_le(&payload[4..8])?;
    let block_align = read_u16_le(&payload[12..14])?;
    let bits_per_sample = read_u16_le(&payload[14..16])?;
    if channels == 0 || sample_rate == 0 || block_align == 0 {
        return Err(Error::AdmUnsupported(
            "invalid fmt chunk: zero channels/sample_rate/block_align".to_string(),
        ));
    }
    if !matches!(bits_per_sample, 16 | 24 | 32) {
        return Err(Error::AdmPcmFormatUnsupported(format!(
            "unsupported bits_per_sample={bits_per_sample}; expected 16/24/32"
        )));
    }

    let is_pcm = if format_tag == 1 {
        true
    } else if format_tag == 0xFFFE {
        if payload.len() < 40 {
            return Err(Error::AdmUnsupported(
                "invalid extensible fmt chunk: payload shorter than 40 bytes".to_string(),
            ));
        }
        let subformat = &payload[24..40];
        // WAVE_FORMAT_EXTENSIBLE subformat GUID tail is fixed.
        let is_pcm_guid_tail =
            subformat[2..] == [0, 0, 0, 0, 16, 0, 128, 0, 0, 170, 0, 56, 155, 113];
        subformat[0] == 1 && subformat[1] == 0 && is_pcm_guid_tail
    } else {
        false
    };

    if !is_pcm {
        return Err(Error::AdmPcmFormatUnsupported(format!(
            "unsupported PCM format tag={format_tag}; only integer PCM is supported"
        )));
    }

    let bytes_per_sample = bits_per_sample / 8;
    let expected_block = channels
        .checked_mul(bytes_per_sample)
        .ok_or_else(|| Error::AdmUnsupported("block_align overflow".to_string()))?;
    if expected_block != block_align {
        return Err(Error::AdmPcmFormatUnsupported(format!(
            "block_align mismatch: got {block_align}, expected {expected_block}"
        )));
    }

    Ok(PcmFormat {
        channels,
        sample_rate,
        bits_per_sample,
        block_align,
        bytes_per_sample,
    })
}

fn read_chunk_header(file: &mut File, header_offset: u64) -> Result<([u8; 4], u32)> {
    let mut header = [0_u8; 8];
    file.seek(SeekFrom::Start(header_offset))
        .map_err(|e| Error::AdmUnsupported(format!("failed to seek chunk header: {e}")))?;
    file.read_exact(&mut header)
        .map_err(|e| Error::AdmUnsupported(format!("failed to read chunk header: {e}")))?;
    Ok((
        [header[0], header[1], header[2], header[3]],
        read_u32_le(&header[4..8])?,
    ))
}

fn read_u16_le(input: &[u8]) -> Result<u16> {
    if input.len() < 2 {
        return Err(Error::AdmUnsupported(
            "failed to read u16 from short buffer".to_string(),
        ));
    }
    Ok(u16::from_le_bytes([input[0], input[1]]))
}

fn read_u32_le(input: &[u8]) -> Result<u32> {
    if input.len() < 4 {
        return Err(Error::AdmUnsupported(
            "failed to read u32 from short buffer".to_string(),
        ));
    }
    Ok(u32::from_le_bytes([input[0], input[1], input[2], input[3]]))
}

fn read_u64_le(input: &[u8]) -> Result<u64> {
    if input.len() < 8 {
        return Err(Error::AdmUnsupported(
            "failed to read u64 from short buffer".to_string(),
        ));
    }
    Ok(u64::from_le_bytes([
        input[0], input[1], input[2], input[3], input[4], input[5], input[6], input[7],
    ]))
}

fn checked_add(base: u64, add: u64, context: &str) -> Result<u64> {
    base.checked_add(add)
        .ok_or_else(|| Error::AdmUnsupported(context.to_string()))
}

fn fourcc_to_string(id: [u8; 4]) -> String {
    String::from_utf8_lossy(&id).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;
    use std::path::PathBuf;

    fn write_temp_bytes(prefix: &str, data: &[u8]) -> Result<PathBuf> {
        let path = std::env::temp_dir().join(format!(
            "{prefix}_{}_{}.wav",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let mut file = File::create(&path)?;
        file.write_all(data)?;
        Ok(path)
    }

    fn push_chunk(dst: &mut Vec<u8>, id: [u8; 4], payload: &[u8], override_size: Option<u32>) {
        dst.extend_from_slice(&id);
        let size_u32 = override_size.unwrap_or_else(|| u32::try_from(payload.len()).unwrap_or(0));
        dst.extend_from_slice(&size_u32.to_le_bytes());
        dst.extend_from_slice(payload);
        if payload.len() % 2 == 1 {
            dst.push(0);
        }
    }

    fn build_test_wave(adm: bool, bw64: bool) -> Vec<u8> {
        let mut fmt = Vec::new();
        fmt.extend_from_slice(&1_u16.to_le_bytes()); // PCM
        fmt.extend_from_slice(&2_u16.to_le_bytes()); // channels
        fmt.extend_from_slice(&48_000_u32.to_le_bytes());
        fmt.extend_from_slice(&(48_000_u32 * 2 * 3).to_le_bytes()); // byte rate
        fmt.extend_from_slice(&6_u16.to_le_bytes()); // block align
        fmt.extend_from_slice(&24_u16.to_le_bytes()); // bps

        let data_payload = vec![
            // 2 frames, 2ch, 24bit
            1, 0, 0, 2, 0, 0, // frame 0 L/R
            3, 0, 0, 4, 0, 0, // frame 1 L/R
        ];

        let mut chunks = Vec::new();
        push_chunk(&mut chunks, FMT_SIG, &fmt, None);
        push_chunk(&mut chunks, *b"zzzz", &[10, 20, 30, 40, 50], None);
        if adm {
            push_chunk(&mut chunks, BEXT_SIG, b"bextv1", None);
            push_chunk(&mut chunks, AXML_SIG, b"<adm/>", None);
            push_chunk(&mut chunks, CHNA_SIG, &[1, 0, 0, 0], None);
        }
        if bw64 {
            push_chunk(&mut chunks, DATA_SIG, &data_payload, Some(u32::MAX));
        } else {
            push_chunk(&mut chunks, DATA_SIG, &data_payload, None);
        }

        if !bw64 {
            let mut out = Vec::new();
            out.extend_from_slice(&RIFF_SIG);
            let riff_size = u32::try_from(chunks.len() + 4).unwrap_or(0);
            out.extend_from_slice(&riff_size.to_le_bytes());
            out.extend_from_slice(&WAVE_SIG);
            out.extend_from_slice(&chunks);
            return out;
        }

        let ds64_payload_len = 28_u32;
        let ds64_total = 8_u64 + u64::from(ds64_payload_len);
        let chunks_total = u64::try_from(chunks.len()).unwrap_or(0);
        let riff_size64 = 4_u64 + ds64_total + chunks_total;
        let data_size64 = u64::try_from(data_payload.len()).unwrap_or(0);
        let sample_count = data_size64 / 6;

        let mut out = Vec::new();
        out.extend_from_slice(&BW64_SIG);
        out.extend_from_slice(&u32::MAX.to_le_bytes());
        out.extend_from_slice(&WAVE_SIG);
        out.extend_from_slice(&DS64_SIG);
        out.extend_from_slice(&ds64_payload_len.to_le_bytes());
        out.extend_from_slice(&riff_size64.to_le_bytes());
        out.extend_from_slice(&data_size64.to_le_bytes());
        out.extend_from_slice(&sample_count.to_le_bytes());
        out.extend_from_slice(&0_u32.to_le_bytes()); // table length
        out.extend_from_slice(&chunks);
        out
    }

    #[test]
    fn parse_riff_adm_chunk_index() {
        let bytes = build_test_wave(true, false);
        let path_result = write_temp_bytes("adm_bwav_riff", &bytes);
        assert!(path_result.is_ok());
        let Ok(path) = path_result else {
            return;
        };

        let parsed = parse_chunk_index(&path);
        assert!(parsed.is_ok());
        let Ok(parsed_opt) = parsed else {
            let _ = fs::remove_file(&path);
            return;
        };
        assert!(parsed_opt.is_some());
        let Some(index) = parsed_opt else {
            let _ = fs::remove_file(&path);
            return;
        };

        assert_eq!(index.container, WaveContainer::Riff);
        assert!(index.has_axml);
        assert!(index.has_chna);
        assert!(index.has_bext);
        assert_eq!(index.fmt.channels, 2);
        assert_eq!(index.fmt.bits_per_sample, 24);
        assert_eq!(index.data_chunk.size, 12);
        assert!(index.chunks.iter().any(|c| c.id == *b"zzzz"));

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn probe_detects_adm_bwf() {
        let bytes = build_test_wave(true, false);
        let path_result = write_temp_bytes("adm_bwav_probe", &bytes);
        assert!(path_result.is_ok());
        let Ok(path) = path_result else {
            return;
        };

        let probed = probe_adm_bwf(&path);
        assert!(probed.is_ok());
        let Ok(found) = probed else {
            let _ = fs::remove_file(&path);
            return;
        };
        assert!(found.is_some());
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn parse_bw64_data_size_from_ds64() {
        let bytes = build_test_wave(true, true);
        let path_result = write_temp_bytes("adm_bwav_bw64", &bytes);
        assert!(path_result.is_ok());
        let Ok(path) = path_result else {
            return;
        };

        let parsed = parse_chunk_index(&path);
        assert!(parsed.is_ok());
        let Ok(parsed_opt) = parsed else {
            let _ = fs::remove_file(&path);
            return;
        };
        assert!(parsed_opt.is_some());
        let Some(index) = parsed_opt else {
            let _ = fs::remove_file(&path);
            return;
        };
        assert_eq!(index.container, WaveContainer::Bw64);
        assert_eq!(index.data_chunk.size, 12);
        assert!(index.data_chunk.padded_size >= index.data_chunk.size);

        let _ = fs::remove_file(&path);
    }
}
