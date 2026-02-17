/**
 * AWMKit - Audio Watermark Kit
 *
 * C header for FFI bindings
 */

#ifndef AWMKIT_H
#define AWMKIT_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/**
 * Error codes
 */
typedef enum {
    AWM_SUCCESS = 0,
    AWM_ERROR_INVALID_TAG = -1,
    AWM_ERROR_INVALID_MESSAGE_LENGTH = -2,
    AWM_ERROR_HMAC_MISMATCH = -3,
    AWM_ERROR_NULL_POINTER = -4,
    AWM_ERROR_INVALID_UTF8 = -5,
    AWM_ERROR_CHECKSUM_MISMATCH = -6,
    AWM_ERROR_AUDIOWMARK_NOT_FOUND = -7,
    AWM_ERROR_AUDIOWMARK_EXEC = -8,
    AWM_ERROR_NO_WATERMARK_FOUND = -9,
    AWM_ERROR_KEY_ALREADY_EXISTS = -10,
    AWM_ERROR_INVALID_OUTPUT_FORMAT = -11,
    AWM_ERROR_ADM_UNSUPPORTED = -12,
    AWM_ERROR_ADM_PRESERVE_FAILED = -13,
    AWM_ERROR_ADM_PCM_FORMAT_UNSUPPORTED = -14,
} AWMError;

/**
 * Decoded message result
 */
typedef struct {
    uint8_t version;
    uint64_t timestamp_utc;     // Unix timestamp in seconds
    uint32_t timestamp_minutes; // Raw value (Unix minutes)
    uint8_t key_slot;           // Key slot (v1: 0, v2: 0-31)
    char tag[9];                // 8 chars + null terminator
    char identity[8];           // 7 chars max + null terminator
} AWMResult;

/**
 * Message length constant
 */
#define AWM_MESSAGE_LENGTH 16

/**
 * Tag length constant (excluding null terminator)
 */
#define AWM_TAG_LENGTH 8

// ============================================================================
// Tag Operations
// ============================================================================

/**
 * Create a new tag from identity string (auto-padding + checksum)
 *
 * @param identity  Identity string (1-7 characters)
 * @param out       Output buffer (at least 9 bytes for 8 chars + null)
 * @return          AWM_SUCCESS or error code
 *
 * Example:
 *   char tag[9];
 *   awm_tag_new("SAKUZY", tag);  // tag = "SAKUZY_X"
 */
int32_t awm_tag_new(const char* identity, char* out);

/**
 * Verify tag checksum
 *
 * @param tag  8-character tag string
 * @return     true if valid, false otherwise
 */
bool awm_tag_verify(const char* tag);

/**
 * Extract identity from tag (without padding and checksum)
 *
 * @param tag  8-character tag string
 * @param out  Output buffer (at least 8 bytes)
 * @return     AWM_SUCCESS or error code
 */
int32_t awm_tag_identity(const char* tag, char* out);

// ============================================================================
// Message Operations
// ============================================================================

/**
 * Encode a watermark message
 *
 * @param version  Protocol version (use 2)
 * @param tag      8-character tag string
 * @param key      HMAC key bytes
 * @param key_len  Key length
 * @param out      Output buffer (at least 16 bytes)
 * @return         AWM_SUCCESS or error code
 */
int32_t awm_message_encode(
    uint8_t version,
    const char* tag,
    const uint8_t* key,
    size_t key_len,
    uint8_t* out
);

/**
 * Encode a watermark message with specific key slot.
 *
 * @param version   Protocol version
 * @param tag       8-character tag string
 * @param key       HMAC key bytes
 * @param key_len   Key length
 * @param key_slot  Key slot (0-31 for v2, 0 for v1)
 * @param out       Output buffer (at least 16 bytes)
 * @return          AWM_SUCCESS or error code
 */
int32_t awm_message_encode_with_slot(
    uint8_t version,
    const char* tag,
    const uint8_t* key,
    size_t key_len,
    uint8_t key_slot,
    uint8_t* out
);

/**
 * Encode a watermark message with specific timestamp
 *
 * @param version           Protocol version
 * @param tag               8-character tag string
 * @param key               HMAC key bytes
 * @param key_len           Key length
 * @param timestamp_minutes UTC Unix minutes
 * @param out               Output buffer (at least 16 bytes)
 * @return                  AWM_SUCCESS or error code
 */
int32_t awm_message_encode_with_timestamp(
    uint8_t version,
    const char* tag,
    const uint8_t* key,
    size_t key_len,
    uint32_t timestamp_minutes,
    uint8_t* out
);

/**
 * Decode and verify a watermark message
 *
 * @param data     16-byte message
 * @param key      HMAC key bytes
 * @param key_len  Key length
 * @param result   Output result structure
 * @return         AWM_SUCCESS, AWM_ERROR_HMAC_MISMATCH, or other error
 */
int32_t awm_message_decode(
    const uint8_t* data,
    const uint8_t* key,
    size_t key_len,
    AWMResult* result
);

/**
 * Decode watermark message fields without HMAC verification.
 *
 * @param data     16-byte message
 * @param result   Output result structure
 * @return         AWM_SUCCESS or error code
 */
int32_t awm_message_decode_unverified(
    const uint8_t* data,
    AWMResult* result
);

/**
 * Verify message HMAC only (without full decoding)
 *
 * @param data     16-byte message
 * @param key      HMAC key bytes
 * @param key_len  Key length
 * @return         true if HMAC is valid
 */
bool awm_message_verify(
    const uint8_t* data,
    const uint8_t* key,
    size_t key_len
);

// ============================================================================
// Utility Functions
// ============================================================================

/**
 * Get current protocol version
 */
uint8_t awm_current_version(void);

/**
 * Get message length constant
 */
size_t awm_message_length(void);

// ============================================================================
// Audio Operations (requires audiowmark binary)
// ============================================================================

/**
 * Opaque audio handle
 */
typedef struct AWMAudioHandle AWMAudioHandle;

/**
 * Audio detection result
 */
typedef struct {
    bool found;                // Whether watermark was found
    uint8_t raw_message[16];   // Extracted message (if found)
    char pattern[16];          // Detection pattern (e.g., "all", "single")
    bool has_detect_score;     // Whether detect_score is available
    float detect_score;        // Detection score from audiowmark
    uint32_t bit_errors;       // Number of bit errors
} AWMDetectResult;

typedef struct {
    char backend[16];          // media backend name
    bool eac3_decode;          // eac3 decoder available
    bool container_mp4;        // supports mp4/m4a container
    bool container_mkv;        // supports mkv/mka container
    bool container_ts;         // supports mpegts container
} AWMAudioMediaCapabilities;

typedef enum {
    AWM_PROGRESS_OP_NONE = 0,
    AWM_PROGRESS_OP_EMBED = 1,
    AWM_PROGRESS_OP_DETECT = 2,
} AWMProgressOperation;

typedef enum {
    AWM_PROGRESS_PHASE_IDLE = 0,
    AWM_PROGRESS_PHASE_PREPARE_INPUT = 1,
    AWM_PROGRESS_PHASE_PRECHECK = 2,
    AWM_PROGRESS_PHASE_CORE = 3,
    AWM_PROGRESS_PHASE_ROUTE_STEP = 4,
    AWM_PROGRESS_PHASE_MERGE = 5,
    AWM_PROGRESS_PHASE_EVIDENCE = 6,
    AWM_PROGRESS_PHASE_CLONE_CHECK = 7,
    AWM_PROGRESS_PHASE_FINALIZE = 8,
} AWMProgressPhase;

typedef enum {
    AWM_PROGRESS_STATE_IDLE = 0,
    AWM_PROGRESS_STATE_RUNNING = 1,
    AWM_PROGRESS_STATE_COMPLETED = 2,
    AWM_PROGRESS_STATE_FAILED = 3,
} AWMProgressState;

typedef struct {
    AWMProgressOperation operation;
    AWMProgressPhase phase;
    AWMProgressState state;
    bool determinate;
    uint64_t completed_units;
    uint64_t total_units;
    uint32_t step_index;
    uint32_t step_total;
    uint64_t op_id;
    char phase_label[64];
} AWMProgressSnapshot;

typedef void (*AWMProgressCallback)(
    const AWMProgressSnapshot* snapshot,
    void* user_data
);

/**
 * Multichannel layout
 */
typedef enum {
    AWM_CHANNEL_LAYOUT_STEREO = 0,
    AWM_CHANNEL_LAYOUT_SURROUND_51 = 1,
    AWM_CHANNEL_LAYOUT_SURROUND_512 = 2,
    AWM_CHANNEL_LAYOUT_SURROUND_71 = 3,
    AWM_CHANNEL_LAYOUT_SURROUND_714 = 4,
    AWM_CHANNEL_LAYOUT_SURROUND_916 = 5,
    AWM_CHANNEL_LAYOUT_AUTO = -1,
} AWMChannelLayout;

/**
 * Multichannel pair detection result
 */
typedef struct {
    uint32_t pair_index;      // Pair index
    bool found;               // Whether watermark was found
    uint8_t raw_message[16];  // Extracted message
    uint32_t bit_errors;      // Bit errors
} AWMPairResult;

/**
 * Multichannel detection result
 */
typedef struct {
    uint32_t pair_count;            // Number of detected pairs
    AWMPairResult pairs[8];         // Pair results (max 8)
    bool has_best;                  // Whether best result exists
    uint8_t best_raw_message[16];   // Best result message
    char best_pattern[16];          // Best detection pattern
    bool has_best_detect_score;     // Whether best detect score is available
    float best_detect_score;        // Best detect score from audiowmark
    uint32_t best_bit_errors;       // Best result bit errors
} AWMMultichannelDetectResult;

typedef enum {
    AWM_CLONE_CHECK_EXACT = 0,
    AWM_CLONE_CHECK_LIKELY = 1,
    AWM_CLONE_CHECK_SUSPECT = 2,
    AWM_CLONE_CHECK_UNAVAILABLE = 3,
} AWMCloneCheckKind;

typedef struct {
    AWMCloneCheckKind kind;    // Clone check status
    bool has_score;            // Whether score is available
    double score;              // Fingerprint score (lower is better)
    bool has_match_seconds;    // Whether match duration is available
    float match_seconds;       // Match duration in seconds
    bool has_evidence_id;      // Whether matched evidence id is available
    int64_t evidence_id;       // Matched evidence id
    char reason[128];          // Optional reason string
} AWMCloneCheckResult;

typedef struct {
    bool has_snr_db;           // Whether snr_db is available
    double snr_db;             // SNR in dB
    char snr_status[16];       // "ok" | "unavailable" | "error"
    char snr_detail[128];      // Optional detail (e.g. mismatch reason)
} AWMEmbedEvidenceResult;

/**
 * Create Audio instance (auto-search for audiowmark)
 *
 * @return  Handle or NULL if audiowmark not found
 */
AWMAudioHandle* awm_audio_new(void);

/**
 * Create Audio instance with specific audiowmark path
 *
 * @param binary_path  Path to audiowmark binary
 * @return             Handle or NULL if path invalid
 */
AWMAudioHandle* awm_audio_new_with_binary(const char* binary_path);

/**
 * Free Audio instance
 *
 * @param handle  Handle to free
 */
void awm_audio_free(AWMAudioHandle* handle);

/**
 * Set watermark strength (1-30, default: 10)
 *
 * @param handle    Audio handle
 * @param strength  Watermark strength
 */
void awm_audio_set_strength(AWMAudioHandle* handle, uint8_t strength);

/**
 * Set key file for audiowmark
 *
 * @param handle    Audio handle
 * @param key_file  Path to key file
 */
void awm_audio_set_key_file(AWMAudioHandle* handle, const char* key_file);

/**
 * Set progress callback (push mode).
 *
 * Callback may be triggered on worker threads.
 */
int32_t awm_audio_progress_set_callback(
    AWMAudioHandle* handle,
    AWMProgressCallback callback,
    void* user_data
);

/**
 * Get latest progress snapshot (poll mode).
 */
int32_t awm_audio_progress_get(
    const AWMAudioHandle* handle,
    AWMProgressSnapshot* result
);

/**
 * Clear current progress state back to idle.
 */
void awm_audio_progress_clear(AWMAudioHandle* handle);

/**
 * Embed watermark into audio file
 *
 * @param handle   Audio handle
 * @param input    Input audio file path
 * @param output   Output audio file path
 * @param message  16-byte message to embed
 * @return         AWM_SUCCESS or error code
 */
int32_t awm_audio_embed(
    const AWMAudioHandle* handle,
    const char* input,
    const char* output,
    const uint8_t* message
);

/**
 * Detect watermark from audio file
 *
 * @param handle  Audio handle
 * @param input   Input audio file path
 * @param result  Output detection result
 * @return        AWM_SUCCESS, AWM_ERROR_NO_WATERMARK_FOUND, or error code
 */
int32_t awm_audio_detect(
    const AWMAudioHandle* handle,
    const char* input,
    AWMDetectResult* result
);

/**
 * Embed watermark with multichannel routing
 *
 * Requires rust feature: multichannel
 */
int32_t awm_audio_embed_multichannel(
    const AWMAudioHandle* handle,
    const char* input,
    const char* output,
    const uint8_t* message,
    AWMChannelLayout layout
);

/**
 * Detect watermark with multichannel routing
 *
 * Requires rust feature: multichannel
 */
int32_t awm_audio_detect_multichannel(
    const AWMAudioHandle* handle,
    const char* input,
    AWMChannelLayout layout,
    AWMMultichannelDetectResult* result
);

/**
 * Get number of channels for a layout
 */
uint32_t awm_channel_layout_channels(AWMChannelLayout layout);

/**
 * Evaluate clone check for a decoded identity/key_slot and input file
 *
 * @param input     Input audio file path
 * @param identity  Decoded identity
 * @param key_slot  Decoded key slot
 * @param result    Output clone check result
 * @return          AWM_SUCCESS or error code
 */
int32_t awm_clone_check_for_file(
    const char* input,
    const char* identity,
    uint8_t key_slot,
    AWMCloneCheckResult* result
);

/**
 * Record evidence for embedded output file
 *
 * @param file_path   Embedded output audio file path
 * @param raw_message 16-byte encoded message
 * @param key         HMAC key bytes
 * @param key_len     Key length
 * @return            AWM_SUCCESS or error code
 */
int32_t awm_evidence_record_file(
    const char* file_path,
    const uint8_t* raw_message,
    const uint8_t* key,
    size_t key_len
);

/**
 * Record evidence for embedded output file with legacy forced flag.
 *
 * @param file_path        Embedded output audio file path
 * @param raw_message      16-byte encoded message
 * @param key              HMAC key bytes
 * @param key_len          Key length
 * @param is_forced_embed  Legacy argument (ignored, retained for ABI compatibility)
 * @return                 AWM_SUCCESS or error code
 */
int32_t awm_evidence_record_file_ex(
    const char* file_path,
    const uint8_t* raw_message,
    const uint8_t* key,
    size_t key_len,
    bool is_forced_embed
);

/**
 * Record evidence for embedded output file and calculate SNR.
 *
 * @param input_path        Original input audio file path
 * @param output_path       Embedded output audio file path
 * @param raw_message       16-byte encoded message
 * @param key               HMAC key bytes
 * @param key_len           Key length
 * @param is_forced_embed   Legacy argument (ignored, retained for ABI compatibility)
 * @param result            Output SNR result payload
 * @return                  AWM_SUCCESS or error code
 */
int32_t awm_evidence_record_embed_file_ex(
    const char* input_path,
    const char* output_path,
    const uint8_t* raw_message,
    const uint8_t* key,
    size_t key_len,
    bool is_forced_embed,
    AWMEmbedEvidenceResult* result
);

/**
 * Check if audiowmark is available
 *
 * @param handle  Audio handle
 * @return        true if audiowmark can be executed
 */
bool awm_audio_is_available(const AWMAudioHandle* handle);

/**
 * Get audiowmark binary path
 *
 * @param handle   Audio handle
 * @param out      Output buffer for path string
 * @param out_len  Buffer capacity in bytes
 * @return         AWM_SUCCESS or error code
 */
int32_t awm_audio_binary_path(
    const AWMAudioHandle* handle,
    char* out,
    size_t out_len
);

/**
 * Query media decode capabilities.
 *
 * @param handle  Audio handle
 * @param result  Output capability struct
 * @return        AWM_SUCCESS or error code
 */
int32_t awm_audio_media_capabilities(
    const AWMAudioHandle* handle,
    AWMAudioMediaCapabilities* result
);

// ============================================================================
// UI Settings (requires "app" feature at build time)
// ============================================================================

/**
 * Get persisted UI language override.
 *
 * Two-step usage:
 * 1) call with out = NULL and out_len = 0 to get out_required_len
 * 2) allocate buffer and call again to fetch UTF-8 language string
 *
 * Returns empty string when unset.
 * Supported values: "zh-CN", "en-US".
 *
 * @param out               Output buffer for UTF-8 language string
 * @param out_len           Buffer capacity in bytes
 * @param out_required_len  Required bytes (includes null terminator)
 * @return                  AWM_SUCCESS or error code
 */
int32_t awm_ui_language_get(char* out, size_t out_len, size_t* out_required_len);

/**
 * Set persisted UI language override.
 *
 * @param lang_or_null  UTF-8 language string ("zh-CN" | "en-US"), or NULL/"" to clear
 * @return              AWM_SUCCESS or error code
 */
int32_t awm_ui_language_set(const char* lang_or_null);

// ============================================================================
// Key Management (requires "app" feature at build time)
// ============================================================================

/**
 * Check if a signing key exists
 *
 * @return  true if key is stored
 */
bool awm_key_exists(void);

/**
 * Get active key backend label
 *
 * On Windows, possible values:
 * - "keyring (service: ...)"
 * - "dpapi (...path...)"
 * - "none" (no key configured)
 *
 * @param out      Output buffer for backend label string
 * @param out_len  Buffer capacity in bytes
 * @return         AWM_SUCCESS or error code
 */
int32_t awm_key_backend_label(char* out, size_t out_len);

/**
 * Load the signing key
 *
 * @param out_key      Output buffer (at least 32 bytes)
 * @param out_key_cap  Buffer capacity (must be >= 32)
 * @return             AWM_SUCCESS or error code
 */
int32_t awm_key_load(uint8_t* out_key, size_t out_key_cap);

/**
 * Generate a new signing key, save it, and return it
 *
 * @param out_key      Output buffer (at least 32 bytes)
 * @param out_key_cap  Buffer capacity (must be >= 32)
 * @return             AWM_SUCCESS or error code
 */
int32_t awm_key_generate_and_save(uint8_t* out_key, size_t out_key_cap);

/**
 * Get current active key slot.
 *
 * @param out_slot  Output pointer for active slot value (0..31)
 * @return          AWM_SUCCESS or error code
 */
int32_t awm_key_active_slot_get(uint8_t* out_slot);

/**
 * Set current active key slot.
 *
 * @param slot  Slot index (0..31)
 * @return      AWM_SUCCESS or error code
 */
int32_t awm_key_active_slot_set(uint8_t slot);

/**
 * Set human-readable label for a slot.
 *
 * @param slot   Slot index (0..31)
 * @param label  UTF-8 label text (non-empty)
 * @return       AWM_SUCCESS or error code
 */
int32_t awm_key_slot_label_set(uint8_t slot, const char* label);

/**
 * Clear human-readable label for a slot.
 *
 * @param slot  Slot index (0..31)
 * @return      AWM_SUCCESS or error code
 */
int32_t awm_key_slot_label_clear(uint8_t slot);

/**
 * Check if a key exists in the specific slot.
 *
 * @param slot  Slot index (0..31)
 * @return      true if configured
 */
bool awm_key_exists_slot(uint8_t slot);

/**
 * Generate and save key into a specific slot.
 *
 * @param slot         Slot index (0..31)
 * @param out_key      Output buffer (at least 32 bytes)
 * @param out_key_cap  Buffer capacity (must be >= 32)
 * @return             AWM_SUCCESS or error code
 */
int32_t awm_key_generate_and_save_slot(uint8_t slot, uint8_t* out_key, size_t out_key_cap);

/**
 * Delete key in specific slot and return effective active slot after fallback.
 *
 * @param slot                 Slot index (0..31)
 * @param out_new_active_slot  Output pointer for effective active slot after delete
 * @return                     AWM_SUCCESS or error code
 */
int32_t awm_key_delete_slot(uint8_t slot, uint8_t* out_new_active_slot);

/**
 * List all key slot summaries as JSON.
 * Two-step usage:
 * 1) call with out = NULL and out_len = 0 to get out_required_len
 * 2) allocate buffer and call again to fetch JSON payload
 *
 * JSON fields per item:
 * - slot
 * - is_active
 * - has_key
 * - key_id (nullable)
 * - label (nullable)
 * - evidence_count
 * - last_evidence_at (nullable)
 * - status_text
 * - duplicate_of_slots
 *
 * @param out               Output buffer for JSON UTF-8
 * @param out_len           Buffer capacity in bytes
 * @param out_required_len  Required bytes (includes null terminator)
 * @return                  AWM_SUCCESS or error code
 */
int32_t awm_key_slot_summaries_json(char* out, size_t out_len, size_t* out_required_len);

/**
 * Delete the stored signing key
 *
 * @return  AWM_SUCCESS or error code
 */
int32_t awm_key_delete(void);

// ============================================================================
// Database Operations (requires "app" feature at build time)
// ============================================================================

/**
 * Query database summary counts.
 *
 * @param out_tag_count       Output pointer for total tag mappings
 * @param out_evidence_count  Output pointer for total evidence rows
 * @return                    AWM_SUCCESS or error code
 */
int32_t awm_db_summary(uint64_t* out_tag_count, uint64_t* out_evidence_count);

/**
 * List tag mappings as JSON.
 * Two-step usage:
 * 1) call with out = NULL and out_len = 0 to get out_required_len
 * 2) allocate buffer and call again to fetch JSON payload
 *
 * @param limit             Max row count (>=1)
 * @param out               Output buffer for JSON UTF-8
 * @param out_len           Buffer capacity in bytes
 * @param out_required_len  Required bytes (includes null terminator)
 * @return                  AWM_SUCCESS or error code
 */
int32_t awm_db_tag_list_json(uint32_t limit, char* out, size_t out_len, size_t* out_required_len);

/**
 * Lookup tag by username (case-insensitive).
 * Returns empty string when mapping is not found.
 *
 * @param username          Username
 * @param out_tag           Output buffer for tag
 * @param out_len           Buffer capacity in bytes
 * @param out_required_len  Required bytes (includes null terminator)
 * @return                  AWM_SUCCESS or error code
 */
int32_t awm_db_tag_lookup(
    const char* username,
    char* out_tag,
    size_t out_len,
    size_t* out_required_len
);

/**
 * Save mapping only when username does not exist.
 *
 * @param username      Username
 * @param tag           8-char tag
 * @param out_inserted  true if inserted, false when already exists
 * @return              AWM_SUCCESS or error code
 */
int32_t awm_db_tag_save_if_absent(const char* username, const char* tag, bool* out_inserted);

/**
 * Remove tag mappings by usernames JSON array.
 *
 * @param usernames_json  JSON array string, e.g. ["alice","bob"]
 * @param out_deleted     Deleted row count
 * @return                AWM_SUCCESS or error code
 */
int32_t awm_db_tag_remove_json(const char* usernames_json, uint32_t* out_deleted);

/**
 * List evidence rows as JSON.
 * Two-step usage is same as awm_db_tag_list_json.
 *
 * @param limit             Max row count (>=1)
 * @param out               Output buffer for JSON UTF-8
 * @param out_len           Buffer capacity in bytes
 * @param out_required_len  Required bytes (includes null terminator)
 * @return                  AWM_SUCCESS or error code
 */
int32_t awm_db_evidence_list_json(uint32_t limit, char* out, size_t out_len, size_t* out_required_len);

/**
 * Remove evidence rows by ids JSON array.
 *
 * @param ids_json    JSON array string, e.g. [1,2,3]
 * @param out_deleted Deleted row count
 * @return            AWM_SUCCESS or error code
 */
int32_t awm_db_evidence_remove_json(const char* ids_json, uint32_t* out_deleted);

// ============================================================================
// Tag Suggestion (requires "app" feature at build time)
// ============================================================================

/**
 * Generate a suggested tag from a username (SHA256 + Base32)
 *
 * @param username  Username string
 * @param out_tag   Output buffer (at least 9 bytes for 8 chars + null)
 * @return          AWM_SUCCESS or error code
 */
int32_t awm_tag_suggest(const char* username, char* out_tag);

#ifdef __cplusplus
}
#endif

#endif /* AWMKIT_H */
