use crate::app::error::{AppError, Result};
use rusqlite::{params, Connection};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const DEFAULT_CANDIDATE_LIMIT: usize = 200;

#[derive(Debug, Clone)]
pub struct NewAudioEvidence {
    pub file_path: String,
    pub tag: String,
    pub identity: String,
    pub version: u8,
    pub key_slot: u8,
    pub timestamp_minutes: u32,
    pub message_hex: String,
    pub sample_rate: u32,
    pub channels: u32,
    pub sample_count: u64,
    pub pcm_sha256: String,
    pub chromaprint: Vec<u32>,
    pub fp_config_id: u8,
}

#[derive(Debug, Clone)]
pub struct AudioEvidence {
    pub id: i64,
    pub created_at: u64,
    pub file_path: String,
    pub tag: String,
    pub identity: String,
    pub version: u8,
    pub key_slot: u8,
    pub timestamp_minutes: u32,
    pub message_hex: String,
    pub sample_rate: u32,
    pub channels: u32,
    pub sample_count: u64,
    pub pcm_sha256: String,
    pub chromaprint: Vec<u32>,
    pub fp_config_id: u8,
}

pub struct EvidenceStore {
    path: PathBuf,
    conn: Connection,
}

impl EvidenceStore {
    pub fn load() -> Result<Self> {
        let path = db_path()?;
        let conn = open_db(&path)?;
        Ok(Self { path, conn })
    }

    #[cfg(test)]
    fn load_at(path: PathBuf) -> Result<Self> {
        let conn = open_db(&path)?;
        Ok(Self { path, conn })
    }

    pub fn insert(&self, input: &NewAudioEvidence) -> Result<bool> {
        let chromaprint_blob = encode_chromaprint_blob(&input.chromaprint);
        #[allow(clippy::cast_possible_truncation)]
        let fingerprint_len = input.chromaprint.len() as i64;
        let created_at = now_ts()?;
        #[allow(clippy::cast_possible_wrap)]
        let sample_count_i64 = input.sample_count as i64;
        let changed = self.conn.execute(
            "INSERT OR IGNORE INTO audio_evidence (
                created_at, file_path, tag, identity, version, key_slot, timestamp_minutes,
                message_hex, sample_rate, channels, sample_count, pcm_sha256,
                chromaprint_blob, fingerprint_len, fp_config_id
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
            params![
                created_at,
                input.file_path,
                input.tag,
                input.identity,
                i64::from(input.version),
                i64::from(input.key_slot),
                i64::from(input.timestamp_minutes),
                input.message_hex,
                i64::from(input.sample_rate),
                i64::from(input.channels),
                sample_count_i64,
                input.pcm_sha256,
                chromaprint_blob,
                fingerprint_len,
                i64::from(input.fp_config_id),
            ],
        )?;
        Ok(changed > 0)
    }

    pub fn list_candidates(&self, identity: &str, key_slot: u8) -> Result<Vec<AudioEvidence>> {
        self.list_candidates_limited(identity, key_slot, DEFAULT_CANDIDATE_LIMIT)
    }

    pub fn list_candidates_limited(
        &self,
        identity: &str,
        key_slot: u8,
        limit: usize,
    ) -> Result<Vec<AudioEvidence>> {
        #[allow(clippy::cast_possible_wrap)]
        let limit_i64 = limit as i64;
        let mut stmt = self.conn.prepare(
            "SELECT
                id, created_at, file_path, tag, identity, version, key_slot, timestamp_minutes,
                message_hex, sample_rate, channels, sample_count, pcm_sha256,
                chromaprint_blob, fp_config_id
             FROM audio_evidence
             WHERE identity = ?1 AND key_slot = ?2
             ORDER BY created_at DESC
             LIMIT ?3",
        )?;

        let mut rows = stmt.query(params![identity, i64::from(key_slot), limit_i64])?;

        let mut out = Vec::new();
        while let Some(row) = rows.next()? {
            let blob: Vec<u8> = row.get(13)?;
            let chromaprint = decode_chromaprint_blob(&blob)?;
            let created_at_i64: i64 = row.get(1)?;
            let sample_count_i64: i64 = row.get(11)?;
            let version_i64: i64 = row.get(5)?;
            let key_slot_i64: i64 = row.get(6)?;
            let timestamp_minutes_i64: i64 = row.get(7)?;
            let sample_rate_i64: i64 = row.get(9)?;
            let channels_i64: i64 = row.get(10)?;
            let fp_config_id_i64: i64 = row.get(14)?;

            out.push(AudioEvidence {
                id: row.get(0)?,
                #[allow(clippy::cast_sign_loss)]
                created_at: created_at_i64 as u64,
                file_path: row.get(2)?,
                tag: row.get(3)?,
                identity: row.get(4)?,
                #[allow(clippy::cast_possible_truncation)]
                version: version_i64 as u8,
                #[allow(clippy::cast_possible_truncation)]
                key_slot: key_slot_i64 as u8,
                #[allow(clippy::cast_possible_truncation)]
                timestamp_minutes: timestamp_minutes_i64 as u32,
                message_hex: row.get(8)?,
                #[allow(clippy::cast_possible_truncation)]
                sample_rate: sample_rate_i64 as u32,
                #[allow(clippy::cast_possible_truncation)]
                channels: channels_i64 as u32,
                #[allow(clippy::cast_sign_loss)]
                sample_count: sample_count_i64 as u64,
                pcm_sha256: row.get(12)?,
                chromaprint,
                #[allow(clippy::cast_possible_truncation)]
                fp_config_id: fp_config_id_i64 as u8,
            });
        }
        Ok(out)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

pub fn encode_chromaprint_blob(chromaprint: &[u32]) -> Vec<u8> {
    let mut blob = Vec::with_capacity(chromaprint.len() * 4);
    for value in chromaprint {
        blob.extend_from_slice(&value.to_le_bytes());
    }
    blob
}

pub fn decode_chromaprint_blob(blob: &[u8]) -> Result<Vec<u32>> {
    if !blob.len().is_multiple_of(4) {
        return Err(AppError::Message(
            "invalid chromaprint blob length".to_string(),
        ));
    }

    let mut output = Vec::with_capacity(blob.len() / 4);
    for chunk in blob.chunks_exact(4) {
        output.push(u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]));
    }
    Ok(output)
}

fn db_path() -> Result<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        let base = std::env::var_os("LOCALAPPDATA")
            .or_else(|| std::env::var_os("APPDATA"))
            .ok_or_else(|| AppError::Message("LOCALAPPDATA/APPDATA not set".to_string()))?;
        let mut path = PathBuf::from(base);
        path.push("awmkit");
        path.push("awmkit.db");
        Ok(path)
    }

    #[cfg(not(target_os = "windows"))]
    {
        let home = std::env::var_os("HOME")
            .ok_or_else(|| AppError::Message("HOME not set".to_string()))?;
        let mut path = PathBuf::from(home);
        path.push(".awmkit");
        path.push("awmkit.db");
        Ok(path)
    }
}

fn open_db(path: &Path) -> Result<Connection> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let conn = Connection::open(path)?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS audio_evidence (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            created_at INTEGER NOT NULL,
            file_path TEXT NOT NULL,
            tag TEXT NOT NULL,
            identity TEXT NOT NULL,
            version INTEGER NOT NULL,
            key_slot INTEGER NOT NULL,
            timestamp_minutes INTEGER NOT NULL,
            message_hex TEXT NOT NULL,
            sample_rate INTEGER NOT NULL,
            channels INTEGER NOT NULL,
            sample_count INTEGER NOT NULL,
            pcm_sha256 TEXT NOT NULL,
            chromaprint_blob BLOB NOT NULL,
            fingerprint_len INTEGER NOT NULL,
            fp_config_id INTEGER NOT NULL,
            UNIQUE(identity, key_slot, pcm_sha256)
        );
        CREATE INDEX IF NOT EXISTS idx_audio_evidence_identity_slot_created
        ON audio_evidence(identity, key_slot, created_at DESC);",
    )?;
    Ok(conn)
}

fn now_ts() -> Result<u64> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| AppError::Message(format!("clock error: {e}")))?;
    Ok(now.as_secs())
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_ID: AtomicU64 = AtomicU64::new(0);

    fn temp_db_path() -> PathBuf {
        let mut path = std::env::temp_dir();
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        path.push(format!("awmkit-evidence-{nanos}-{id}.db"));
        path
    }

    fn sample_evidence(identity: &str, key_slot: u8, sha256: &str) -> NewAudioEvidence {
        NewAudioEvidence {
            file_path: "/tmp/a.wav".to_string(),
            tag: "ABCDEFGH".to_string(),
            identity: identity.to_string(),
            version: 2,
            key_slot,
            timestamp_minutes: 1234,
            message_hex: "00112233445566778899aabbccddeeff".to_string(),
            sample_rate: 44_100,
            channels: 2,
            sample_count: 10_000,
            pcm_sha256: sha256.to_string(),
            chromaprint: vec![1, 2, 3, 4],
            fp_config_id: 1,
        }
    }

    #[test]
    fn chromaprint_blob_roundtrip() {
        let src = vec![0u32, 1, 42, u32::MAX];
        let blob = encode_chromaprint_blob(&src);
        let decoded = decode_chromaprint_blob(&blob).unwrap();
        assert_eq!(src, decoded);
    }

    #[test]
    fn unique_constraint_ignores_duplicates() {
        let db_path = temp_db_path();
        let store = EvidenceStore::load_at(db_path.clone()).unwrap();
        let first = sample_evidence("TESTER", 0, "abc");
        let second = sample_evidence("TESTER", 0, "abc");
        assert!(store.insert(&first).unwrap());
        assert!(!store.insert(&second).unwrap());

        let candidates = store.list_candidates("TESTER", 0).unwrap();
        assert_eq!(candidates.len(), 1);
        let _ = fs::remove_file(db_path);
    }

    #[test]
    fn list_candidates_filters_by_identity_and_slot() {
        let db_path = temp_db_path();
        let store = EvidenceStore::load_at(db_path.clone()).unwrap();
        let target = sample_evidence("TARGET", 2, "s1");
        let other_id = sample_evidence("OTHER", 2, "s2");
        let other_slot = sample_evidence("TARGET", 1, "s3");
        store.insert(&target).unwrap();
        store.insert(&other_id).unwrap();
        store.insert(&other_slot).unwrap();

        let candidates = store.list_candidates("TARGET", 2).unwrap();
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].identity, "TARGET");
        assert_eq!(candidates[0].key_slot, 2);
        let _ = fs::remove_file(db_path);
    }
}
