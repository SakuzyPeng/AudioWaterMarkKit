cli-error-quiet_verbose_conflict = --quiet and --verbose cannot be used together
cli-error-key_exists = key already exists; use `awmkit key rotate` or `awmkit key import`
cli-error-key_not_found = key not found; run `awmkit init` or `awmkit key import`
cli-error-invalid_key_length = invalid key length: expected { $expected } bytes, got { $actual }
cli-error-key_store = key store error: { $error }
cli-error-audiowmark_not_found = audiowmark not found; use --audiowmark <PATH> or add to PATH
cli-error-input_not_found = input not found: { $path }
cli-error-invalid_glob = invalid glob pattern: { $pattern }
cli-error-glob = glob error: { $error }
cli-error-mapping_exists = mapping exists for { $username }; use --force to overwrite

cli-util-no_input_files = no input files provided

cli-init-ok_generated = [OK] generated key
cli-init-ok_stored = [OK] stored in keyring ({ $bytes } bytes)

cli-key-status_configured = Key status: configured
cli-key-length = Length: { $bytes } bytes
cli-key-fingerprint = Fingerprint (SHA256): { $fingerprint }
cli-key-storage = Storage: { $backend }
cli-key-replaced = [OK] key replaced
cli-key-imported = [OK] key imported
cli-key-exported = [OK] key exported
cli-key-rotated = [OK] key rotated

cli-status-version = awmkit v{ $version }
cli-status-key_configured = Key: configured ({ $bytes } bytes)
cli-status-key_storage = Key storage: { $backend }
cli-status-key_len_mismatch = Key length does not match expected size
cli-status-key_not_configured = Key: not configured
cli-status-audiowmark_available = audiowmark: available
cli-status-audiowmark_not_responding = audiowmark: not responding
cli-status-audiowmark_version = audiowmark version: { $version }
cli-status-audiowmark_version_error = audiowmark version error: { $error }
cli-status-audiowmark_path = audiowmark path: { $path }
cli-status-audiowmark_found = audiowmark: found
cli-status-audiowmark_not_found = audiowmark: not found

cli-embed-output_single = --output only supports a single input file
cli-embed-done = Done: { $success } succeeded, { $failed } failed
cli-embed-failed = one or more files failed

cli-detect-done = Done: { $ok } ok, { $miss } missing, { $invalid } invalid
cli-detect-failed = one or more files failed

cli-decode-version = Version: { $version }
cli-decode-timestamp_minutes = Timestamp (minutes): { $minutes }
cli-decode-timestamp_utc = Timestamp (UTC seconds): { $seconds }
cli-decode-tag = Tag: { $tag }
cli-decode-identity = Identity: { $identity }
cli-decode-status_valid = Status: valid

cli-tag-saved = saved: { $username } -> { $tag }
cli-tag-none = no saved tags
cli-tag-removed = removed: { $username }
cli-tag-cleared = cleared all mappings

ui-window-title = AWMKit GUI
ui-tabs-embed = Embed
ui-tabs-detect = Detect
ui-tabs-status = Status / Init
ui-tabs-tag = Tag Manager

ui-page-embed = Embed
ui-page-detect = Detect
ui-page-status = Status / Init
ui-page-tag = Tag Manager

ui-action-add_files = Add Files
ui-action-clear = Clear
ui-action-apply = Apply
ui-action-run_embed = Run Embed
ui-action-run_detect = Run Detect
ui-action-cancel = Cancel
ui-action-refresh = Refresh
ui-action-init_key = Initialize Key
ui-action-save = Save
ui-action-remove = Remove
ui-action-clear_mappings = Clear
ui-action-refresh_mappings = Refresh
ui-action-overwrite = Overwrite
ui-action-skip = Skip
ui-action-browse = Browse
ui-action-clear_queue = Clear Queue
ui-action-clear_all = Clear All
ui-action-copy_md = Copy Markdown
ui-action-clear_input = Clear Input
ui-action-clear_output = Clear Output
ui-action-start_process = Start Processing
ui-action-clear_log = Clear Log

ui-label-tag = Tag
ui-label-username_optional = Username (optional)
ui-label-username = Username
ui-label-use_mapping = Use mapping
ui-label-strength = Strength
ui-label-language = Language

ui-mapping-select_placeholder = (select mapping)

ui-prompt-save_mapping_title = Save mapping?
ui-prompt-save_mapping_message = No mapping found for this tag.
ui-prompt-mapping_exists = Mapping exists. Click overwrite to replace.
ui-prompt-save_failed = Save failed: { $error }

ui-status-ready = Ready
ui-status-no_input_files = No input files selected.
ui-status-embedding = Embedding...
ui-status-detecting = Detecting...
ui-status-embed_finished = Embed finished.
ui-status-detect_finished = Detect finished.
ui-status-key_initialized = Key initialized.
ui-status-init_failed = Init failed: { $error }
ui-status-mapping_saved = Mapping saved.
ui-status-mapping_not_saved = Mapping not saved.
ui-status-mapping_removed = Mapping removed.
ui-status-mappings_cleared = Mappings cleared.
ui-status-remove_failed = Remove failed: { $error }
ui-status-clear_failed = Clear failed: { $error }
ui-status-save_failed = Save failed: { $error }
ui-status-key_error_short = Key error: { $error }
ui-status-audio_error_short = Audio error: { $error }

ui-status-key_configured = Key: configured ({ $bytes } bytes) | { $backend }
ui-status-key_error = Key: error ({ $error })
ui-status-key_not_configured = Key: not configured
ui-status-audiowmark_ok = audiowmark: { $path } ({ $version })
ui-status-audiowmark_not_available = audiowmark: not available ({ $error })
ui-status-key_not_configured_hint = Key not configured. Initialize in Status / Init.

ui-detect-files-title = Files
ui-detect-log-title = Summary
ui-detect-json-title = JSON details
ui-detect-summary-title = Summary
ui-detect-detail-title = Details
ui-detect-detail-file = File
ui-detect-detail-status = Status
ui-detect-detail-tag = Tag
ui-detect-detail-identity = Identity
ui-detect-detail-version = Version
ui-detect-detail-timestamp = Timestamp
ui-detect-detail-pattern = Pattern
ui-detect-detail-bit_errors = Bit errors
ui-detect-detail-match_found = Match found
ui-detect-detail-error = Error
ui-detect-status-ok = OK
ui-detect-status-found = Found
ui-detect-status-not_found = Not found
ui-detect-status-invalid_hmac = Invalid HMAC
ui-detect-status-error = Error
ui-detect-summary-waiting = Waiting
ui-detect-summary-found = Found
ui-detect-summary-not_found = Not found
ui-detect-summary-unsupported = Unsupported
ui-detect-summary-error = Error

ui-status-card-key = Key
ui-status-card-engine = audiowmark
ui-status-card-actions = Actions
ui-status-card-language = Language

ui-card-embed_settings = Embed Settings
ui-card-file_queue = File Queue
ui-card-result_log = Result Log
ui-card-input_dir = Input Directory
ui-card-output_dir = Output Directory
ui-card-system_status = System Status
ui-card-saved_mappings = Saved Mappings
ui-card-add_mapping = Add Mapping
ui-card-actions = Actions
ui-card-danger_zone = Danger Zone

ui-drop-hint = Drop WAV/FLAC files or folders here
ui-drop-hint-detailed = Drop audio files or directories
ui-drop-hint-sub = Supports mixed input, directories will be recursively extracted
ui-drop-compact = Drop files or

ui-status-no_input = No files added yet
ui-status-no_output = Not selected (default: write back to original directories)

ui-empty-no_files = No files added yet
ui-empty-no_results = Results will appear here
ui-empty-no_tags = No saved mappings yet
ui-empty-detect = Add files to detect watermarks

ui-status-embed_done = Done
ui-status-detect_done = Detection complete

ui-danger-title = Danger zone
ui-danger-expand = Expand
ui-danger-collapse = Collapse
ui-danger-clear_cache = Clear cache/config
ui-danger-clear_cache_title = Clear Cache/Config
ui-danger-reset_all = Reset all (delete key)
ui-danger-reset_all_title = Reset All
ui-danger-clear_tags_title = Clear All Mappings
ui-danger-confirm_title = Confirm action
ui-danger-confirm_message_clear = This will remove cached binaries and config files. Your key will NOT be deleted.
ui-danger-confirm_message_reset = This will delete the key, tags, and local config/cache. This action cannot be undone.
ui-danger-confirm_placeholder = Type { $code } to confirm
ui-danger-confirm_action = Confirm
ui-danger-cancel = Cancel

ui-status-cache_cleared = Cache/config cleared.
ui-status-reset_done = Reset completed.
ui-status-clear_cache_failed = Clear cache failed: { $error }
ui-status-reset_failed = Reset failed: { $error }

ui-error-tag_required = tag is required
ui-error-username_required = username is required
ui-tag-none = no saved tags
ui-error-key_exists = key already exists
ui-tag-load_failed = load failed: { $error }
ui-log-embed_done = Done: { $success } succeeded, { $failed } failed
ui-log-detect_done = Done: { $ok } ok, { $missing } missing, { $invalid } invalid

ui-shell-state-ready = READY
ui-shell-state-running = RUNNING
ui-shell-state-success = SUCCESS
ui-shell-state-error = ERROR
ui-shell-working = Working...

ui-section-file_queue = File queue
ui-section-embed_params = Embed parameters
ui-section-execution = Execution
ui-section-detect_summary = Detection summary
ui-section-tag_table = Saved mappings

ui-empty-files-title = No files yet
ui-empty-files-description = Add audio files to start embedding.
ui-empty-execution = Waiting for execution
ui-empty-detect-summary = No detection result yet
ui-empty-detect-files-title = No files selected
ui-empty-detect-files-description = Select audio files to run detection.
ui-empty-detect-result-title = No selected result
ui-empty-detect-result-description = Run detection and choose a file to inspect details.
ui-empty-tags-title = No saved mappings
ui-empty-tags-description = Save a username and tag mapping to speed up embedding.
ui-label-more_files = more files

ui-status-pending = pending
ui-action-expand_json = Expand JSON
ui-action-suggest = Suggest

ui-status-running_init = Initializing key...
ui-status-running_refresh = Refreshing status...
ui-status-running_language = Updating language...
ui-status-running_clear_cache = Clearing cache...
ui-status-running_reset = Resetting all data...
ui-status-running_suggest = Generating suggested tag...
ui-status-running_save = Saving mapping...
ui-status-running_remove = Removing mapping...
ui-status-running_clear_tags = Clearing mappings...
ui-status-running_refresh_tags = Refreshing mappings...

ui-status-refresh_done = Status refreshed.
ui-status-language_updated = Language updated.
ui-status-suggest_done = Suggested tag generated.
ui-status-username_required = Username is required.
ui-status-cancelled = Cancelled

ui-danger-confirm_reset_hint = Type RESET to confirm.
ui-danger-confirm_awmkit_hint = Type AWMKIT to confirm.
ui-danger-confirm_input_hint = Input RESET / AWMKIT

ui-status-field-state = State
ui-status-field-bytes = Bytes
ui-status-field-backend = Backend
ui-status-field-error = Error
ui-status-field-path = Path
ui-status-field-version = Version
ui-field-state = State
ui-field-bytes = Bytes
ui-field-backend = Backend
ui-field-error = Error
ui-field-path = Path
ui-field-version = Version

ui-table-created = Created At
ui-status-key_title = Key Status

ui-detect-summary-missing = Missing
ui-detect-summary-invalid = Invalid
ui-detect-summary-errors = Errors

ui-toast-copied = Copied to clipboard
ui-toast-tag_saved = Tag mapping saved
ui-toast-tag_removed = Tag mapping removed
ui-toast-tags_cleared = All mappings cleared
ui-toast-cache_cleared = Cache cleared
ui-toast-reset_done = Reset completed

ui-placeholder-username = Input username
ui-placeholder-tag = Input or generate tag
ui-placeholder-tag_optional = (Optional)

ui-confirm-clear_tags = Clear all mappings?

ui-tag-table-username = Username
ui-tag-table-tag = Tag
ui-tag-table-created_at = Created At

ui-label-success = Success
ui-label-failed = Failed

ui-state-configured = Configured
ui-state-not_configured = Not configured
ui-state-error = Error
ui-state-ok = OK
ui-state-not_available = Not available
