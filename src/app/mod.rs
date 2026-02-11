pub mod app_settings_store;
pub mod audio_engine;
pub mod audio_proof;
pub mod error;
pub mod evidence_store;
pub mod i18n;
pub mod keystore;
pub mod maintenance;
pub mod settings;
pub mod snr;
pub mod tag_store;

pub use app_settings_store::{
    is_valid_slot, validate_slot, AppSettingsStore, KEY_SLOT_MAX, KEY_SLOT_MIN,
};
pub use audio_engine::{AppConfig, AudioEngine, DetectOutcome};
pub use audio_proof::{build_audio_proof, AudioProof};
pub use error::{AppError, Result};
pub use evidence_store::{AudioEvidence, EvidenceSlotUsage, EvidenceStore, NewAudioEvidence};
pub use i18n::{
    available_languages, current_language, env_language, set_language, tr, tr_args, LanguageInfo,
};
pub use keystore::{
    generate_key, key_id_from_key_material, KeyBackend, KeySlotSummary, KeyStore, KEY_LEN,
};
pub use maintenance::{clear_local_cache, reset_all};
pub use settings::AppSettings;
pub use snr::{analyze_snr, SnrAnalysis, SNR_STATUS_ERROR, SNR_STATUS_OK, SNR_STATUS_UNAVAILABLE};
pub use tag_store::{TagEntry, TagStore};
