use crate::app::error::{AppError, Result};
use rusqlite::{params, Connection, OptionalExtension, Row};
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

/// Slot usage summary from evidence table.
#[derive(Debug, Clone, Copy)]
pub struct EvidenceSlotUsage {
    /// Number of rows for slot.
    pub count: usize,
    /// Most recent evidence timestamp in unix seconds.
    pub last_created_at: Option<u64>,
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
        self.list_filtered(Some(identity), None, Some(key_slot), limit)
    }

    pub fn list_filtered(
        &self,
        identity: Option<&str>,
        tag: Option<&str>,
        key_slot: Option<u8>,
        limit: usize,
    ) -> Result<Vec<AudioEvidence>> {
        #[allow(clippy::cast_possible_wrap)]
        let limit_i64 = limit as i64;
        let key_slot_i64 = key_slot.map(i64::from);
        let mut stmt = self.conn.prepare(
            "SELECT
                id, created_at, file_path, tag, identity, version, key_slot, timestamp_minutes,
                message_hex, sample_rate, channels, sample_count, pcm_sha256,
                chromaprint_blob, fp_config_id
             FROM audio_evidence
             WHERE (?1 IS NULL OR identity = ?1)
               AND (?2 IS NULL OR tag = ?2)
               AND (?3 IS NULL OR key_slot = ?3)
             ORDER BY created_at DESC
             LIMIT ?4",
        )?;

        let mut rows = stmt.query(params![identity, tag, key_slot_i64, limit_i64])?;

        let mut out = Vec::new();
        while let Some(row) = rows.next()? {
            out.push(parse_audio_evidence_row(row)?);
        }
        Ok(out)
    }

    pub fn get_by_id(&self, id: i64) -> Result<Option<AudioEvidence>> {
        let mut stmt = self.conn.prepare(
            "SELECT
                id, created_at, file_path, tag, identity, version, key_slot, timestamp_minutes,
                message_hex, sample_rate, channels, sample_count, pcm_sha256,
                chromaprint_blob, fp_config_id
             FROM audio_evidence
             WHERE id = ?1
             LIMIT 1",
        )?;
        let mut rows = stmt.query(params![id])?;
        let Some(row) = rows.next()? else {
            return Ok(None);
        };
        Ok(Some(parse_audio_evidence_row(row)?))
    }

    pub fn remove_by_id(&self, id: i64) -> Result<bool> {
        let affected = self
            .conn
            .execute("DELETE FROM audio_evidence WHERE id = ?1", params![id])?;
        Ok(affected > 0)
    }

    pub fn clear_filtered(
        &self,
        identity: Option<&str>,
        tag: Option<&str>,
        key_slot: Option<u8>,
    ) -> Result<usize> {
        let key_slot_i64 = key_slot.map(i64::from);
        let affected = self.conn.execute(
            "DELETE FROM audio_evidence
             WHERE (?1 IS NULL OR identity = ?1)
               AND (?2 IS NULL OR tag = ?2)
               AND (?3 IS NULL OR key_slot = ?3)",
            params![identity, tag, key_slot_i64],
        )?;
        Ok(affected)
    }

    pub fn count_all(&self) -> Result<usize> {
        let mut stmt = self.conn.prepare("SELECT COUNT(*) FROM audio_evidence")?;
        let count: i64 = stmt
            .query_row([], |row| row.get(0))
            .optional()?
            .unwrap_or(0);
        #[allow(clippy::cast_sign_loss)]
        let count = count as usize;
        Ok(count)
    }

    /// Count evidence rows for one key slot.
    pub fn count_by_slot(&self, key_slot: u8) -> Result<usize> {
        let mut stmt = self
            .conn
            .prepare("SELECT COUNT(*) FROM audio_evidence WHERE key_slot = ?1")?;
        let count: i64 = stmt
            .query_row(params![i64::from(key_slot)], |row| row.get(0))
            .optional()?
            .unwrap_or(0);
        #[allow(clippy::cast_sign_loss)]
        let count = count as usize;
        Ok(count)
    }

    /// Usage stats for one key slot.
    pub fn usage_by_slot(&self, key_slot: u8) -> Result<EvidenceSlotUsage> {
        let mut stmt = self.conn.prepare(
            "SELECT COUNT(*), MAX(created_at)
             FROM audio_evidence
             WHERE key_slot = ?1",
        )?;
        let (count_i64, last_i64): (i64, Option<i64>) = stmt
            .query_row(params![i64::from(key_slot)], |row| {
                Ok((row.get(0)?, row.get(1)?))
            })
            .optional()?
            .unwrap_or((0, None));

        #[allow(clippy::cast_sign_loss)]
        let count = count_i64 as usize;
        let last_created_at = last_i64.and_then(|value| u64::try_from(value).ok());

        Ok(EvidenceSlotUsage {
            count,
            last_created_at,
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

fn parse_audio_evidence_row(row: &Row<'_>) -> Result<AudioEvidence> {
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

    Ok(AudioEvidence {
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
    })
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

    #[test]
    fn list_filtered_combines_identity_tag_and_slot() {
        let db_path = temp_db_path();
        let store = EvidenceStore::load_at(db_path.clone()).unwrap();

        let mut target = sample_evidence("TARGET", 2, "a1");
        target.tag = "TAG_A".to_string();
        store.insert(&target).unwrap();

        let mut other_tag = sample_evidence("TARGET", 2, "a2");
        other_tag.tag = "TAG_B".to_string();
        store.insert(&other_tag).unwrap();

        let other_slot = sample_evidence("TARGET", 1, "a3");
        store.insert(&other_slot).unwrap();

        let list = store
            .list_filtered(Some("TARGET"), Some("TAG_A"), Some(2), 50)
            .unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].tag, "TAG_A");
        assert_eq!(list[0].key_slot, 2);
        let _ = fs::remove_file(db_path);
    }

    #[test]
    fn get_and_remove_by_id_work() {
        let db_path = temp_db_path();
        let store = EvidenceStore::load_at(db_path.clone()).unwrap();
        let one = sample_evidence("ONE", 0, "x1");
        store.insert(&one).unwrap();

        let listed = store.list_filtered(Some("ONE"), None, Some(0), 10).unwrap();
        assert_eq!(listed.len(), 1);
        let id = listed[0].id;

        let found = store.get_by_id(id).unwrap();
        assert!(found.is_some());

        assert!(store.remove_by_id(id).unwrap());
        assert!(!store.remove_by_id(id).unwrap());
        assert!(store.get_by_id(id).unwrap().is_none());
        let _ = fs::remove_file(db_path);
    }

    #[test]
    fn clear_filtered_and_count_all_work() {
        let db_path = temp_db_path();
        let store = EvidenceStore::load_at(db_path.clone()).unwrap();

        let mut a = sample_evidence("A", 0, "c1");
        a.tag = "T1".to_string();
        let mut b = sample_evidence("A", 1, "c2");
        b.tag = "T1".to_string();
        let mut c = sample_evidence("B", 0, "c3");
        c.tag = "T2".to_string();
        store.insert(&a).unwrap();
        store.insert(&b).unwrap();
        store.insert(&c).unwrap();

        assert_eq!(store.count_all().unwrap(), 3);
        let removed = store.clear_filtered(Some("A"), Some("T1"), None).unwrap();
        assert_eq!(removed, 2);
        assert_eq!(store.count_all().unwrap(), 1);

        let removed_none = store.clear_filtered(Some("A"), Some("T1"), None).unwrap();
        assert_eq!(removed_none, 0);
        let _ = fs::remove_file(db_path);
    }
}
