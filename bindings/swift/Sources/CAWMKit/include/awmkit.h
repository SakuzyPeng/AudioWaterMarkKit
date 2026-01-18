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
 * @param version  Protocol version (use 1)
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
    uint32_t bit_errors;       // Number of bit errors
} AWMDetectResult;

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
 * Check if audiowmark is available
 *
 * @param handle  Audio handle
 * @return        true if audiowmark can be executed
 */
bool awm_audio_is_available(const AWMAudioHandle* handle);

// ============================================================================
// Multichannel Audio Operations
// ============================================================================

/**
 * Channel layout for multichannel audio
 */
typedef enum {
    AWM_CHANNEL_LAYOUT_STEREO = 0,      // 2 channels
    AWM_CHANNEL_LAYOUT_SURROUND_51 = 1, // 6 channels: FL FR FC LFE BL BR
    AWM_CHANNEL_LAYOUT_SURROUND_512 = 2,// 8 channels: FL FR FC LFE BL BR TFL TFR
    AWM_CHANNEL_LAYOUT_SURROUND_71 = 3, // 8 channels: FL FR FC LFE BL BR SL SR
    AWM_CHANNEL_LAYOUT_SURROUND_714 = 4,// 12 channels
    AWM_CHANNEL_LAYOUT_SURROUND_916 = 5,// 16 channels
    AWM_CHANNEL_LAYOUT_AUTO = -1,       // Auto-detect from file
} AWMChannelLayout;

/**
 * Single channel pair detection result
 */
typedef struct {
    uint32_t pair_index;       // Channel pair index (0-based)
    bool found;                // Whether watermark was found
    uint8_t raw_message[16];   // Extracted message (if found)
    uint32_t bit_errors;       // Number of bit errors
} AWMPairResult;

/**
 * Multichannel detection result
 */
typedef struct {
    uint32_t pair_count;          // Number of channel pairs processed
    AWMPairResult pairs[8];       // Results for each pair (max 8 pairs)
    bool has_best;                // Whether a best result was found
    uint8_t best_raw_message[16]; // Best message (lowest bit errors)
    uint32_t best_bit_errors;     // Best result bit errors
} AWMMultichannelDetectResult;

/**
 * Get number of channels for a layout
 *
 * @param layout  Channel layout
 * @return        Number of channels (0 for auto/unknown)
 */
uint32_t awm_channel_layout_channels(AWMChannelLayout layout);

/**
 * Embed watermark into multichannel audio file
 *
 * @param handle   Audio handle
 * @param input    Input audio file path
 * @param output   Output audio file path
 * @param message  16-byte message to embed
 * @param layout   Channel layout (use AWM_CHANNEL_LAYOUT_AUTO for auto-detect)
 * @return         AWM_SUCCESS or error code
 */
int32_t awm_audio_embed_multichannel(
    const AWMAudioHandle* handle,
    const char* input,
    const char* output,
    const uint8_t* message,
    AWMChannelLayout layout
);

/**
 * Detect watermark from multichannel audio file
 *
 * @param handle  Audio handle
 * @param input   Input audio file path
 * @param layout  Channel layout (use AWM_CHANNEL_LAYOUT_AUTO for auto-detect)
 * @param result  Output detection result
 * @return        AWM_SUCCESS or error code
 */
int32_t awm_audio_detect_multichannel(
    const AWMAudioHandle* handle,
    const char* input,
    AWMChannelLayout layout,
    AWMMultichannelDetectResult* result
);

#ifdef __cplusplus
}
#endif

#endif /* AWMKIT_H */
