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
 * Delete the stored signing key
 *
 * @return  AWM_SUCCESS or error code
 */
int32_t awm_key_delete(void);

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
