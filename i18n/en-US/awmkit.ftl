cli-error-quiet_verbose_conflict = Cannot use --quiet with --verbose. Next: remove one flag and run the command again.
cli-error-key_exists = A key already exists in this slot. Next: run `awmkit key rotate` to replace it, or `awmkit key import` to load another key.
cli-error-key_not_found = No key is configured for this slot. Next: run `awmkit init` or `awmkit key import`.
cli-error-invalid_key_length = Key length is invalid (expected { $expected } bytes, got { $actual }). Next: provide a key with exactly { $expected } bytes.
cli-error-key_store = Key store operation failed. Next: check key backend permissions and retry.
cli-error-audiowmark_not_found = audiowmark is unavailable. Next: set `--audiowmark <PATH>` or add the binary to PATH.
cli-error-input_not_found = Input file was not found: { $path }. Next: verify the path and run again.
cli-error-invalid_glob = Glob pattern is invalid: { $pattern }. Next: fix the pattern or pass explicit input files.
cli-error-glob = Failed to expand the file pattern. Next: verify the path pattern and retry.
cli-error-mapping_exists = Mapping already exists for { $username }. Next: use `--force` to overwrite, or use another username.
cli-error-database = Database operation failed. Next: run `awmkit status --doctor --verbose` and check database permissions.
cli-error-config = Configuration operation failed. Next: check your config path and retry.
cli-error-io = File operation failed. Next: verify file path and permissions.
cli-error-hex = Hex input is invalid. Next: provide an even-length hexadecimal string.
cli-error-audio = Audio processing failed. Next: verify input format support and rerun with `--verbose`.
cli-error-json = JSON parse/serialize failed. Next: rerun with `--json` and validate the payload format.

cli-util-no_input_files = No input files were provided. Next: pass one or more input paths or glob patterns.

cli-init-ok_generated = Key generated for the active slot. Next: run `awmkit key show` to verify key details.
cli-init-ok_stored = Key stored in key backend ({ $bytes } bytes). Next: run `awmkit embed ...` to start embedding.

cli-key-status_configured = Key is configured for the active slot. Next: run `awmkit key show` for fingerprint and backend details.
cli-key-length = Length: { $bytes } bytes
cli-key-fingerprint = Fingerprint (SHA256): { $fingerprint }
cli-key-storage = Storage: { $backend }
cli-key-replaced = Key replaced in the active slot. Next: run `awmkit key show` to confirm fingerprint.
cli-key-imported = Key imported into the active slot. Next: run `awmkit key show` to verify backend and fingerprint.
cli-key-exported = Key exported from the active slot. Next: store the exported file securely.
cli-key-rotated = Key rotated in the active slot. Next: rerun `awmkit detect <file>` on recent outputs if needed.
cli-key-slot = Slot: { $slot }
cli-key-slot-active = This is the active slot. Next: run `awmkit key slot use <slot>` to switch if needed.
cli-key-slot-current_active = Current active slot: { $slot }
cli-key-imported-slot = Key imported into slot { $slot }. Next: run `awmkit key slot use { $slot }` if this slot should be active.
cli-key-exported-slot = Key exported from slot { $slot }. Next: keep the exported file in secure storage.
cli-key-rotated-slot = Key rotated for slot { $slot }. Next: verify slot status with `awmkit key slot list`.
cli-key-delete-requires-yes = Key deletion was not executed. Next: rerun with `--yes` to confirm deletion.
cli-key-delete-slot-has-evidence = Slot { $slot } still has { $count } evidence records. Next: clear evidence for this slot first, or rerun with `--force`.
cli-key-deleted-slot = Key deleted from slot { $slot }. Next: run `awmkit key import --slot { $slot }` or `awmkit key rotate --slot { $slot }` to configure a new key.
cli-key-slot-current = Active slot: { $slot }
cli-key-slot-set = Active slot switched to { $slot }. Next: run `awmkit key show` to confirm key details for this slot.
cli-key-slot-label-set = Slot { $slot } label updated to { $label }. Next: run `awmkit key slot list` to review all labels.
cli-key-slot-label-cleared = Slot { $slot } label cleared. Next: run `awmkit key slot label set { $slot } <label>` to set a new label.
cli-key-conflict-slot-occupied = Slot conflict detected: slot { $slot } already has a key. Next: delete that slot key first or choose another slot.
cli-key-conflict-slot-has-evidence = Slot conflict detected: slot { $slot } still has { $count } evidence records. Next: clear evidence for this slot before continuing.
cli-key-conflict-duplicate-fingerprint = Slot conflict detected: target slot { $slot } duplicates fingerprint in slot(s) { $conflicts }. Next: clear conflicting slot keys first.
cli-key-slot-state-configured = configured
cli-key-slot-state-empty = empty
cli-key-slot-active-marker = active
cli-key-slot-inactive-marker = inactive
cli-key-slot-list-row = Slot { $slot } [{ $active }]: { $state }, label={ $label }, fp={ $fingerprint }, backend={ $backend }, evidence={ $evidence }, last={ $last }
cli-key-error-invalid-slot-input = Slot value is invalid: { $input }. Next: pass an integer slot id.
cli-key-error-invalid-slot-range = Slot { $slot } is out of range. Next: choose a slot in 0..={ $max }.

cli-status-version = awmkit v{ $version }
cli-status-key_configured = Key is configured ({ $bytes } bytes). Next: run `awmkit key show` for fingerprint details.
cli-status-key_storage = Key backend is { $backend }. Next: keep this backend available for key operations.
cli-status-key_len_mismatch = Key length does not match expected size. Next: import or rotate a valid key.
cli-status-key_not_configured = Key is not configured. Next: run `awmkit init` or `awmkit key import`.
cli-status-audiowmark_available = audiowmark is available. Next: run `awmkit detect <file>` to verify the full pipeline.
cli-status-audiowmark_not_responding = audiowmark is not responding. Next: check binary permissions and path.
cli-status-audiowmark_version = audiowmark version: { $version }
cli-status-audiowmark_version_error = audiowmark version error: { $error }
cli-status-audiowmark_path = audiowmark path: { $path }
cli-status-audiowmark_found = audiowmark binary found. Next: run `awmkit status --doctor` if runtime issues remain.
cli-status-audiowmark_not_found = audiowmark binary not found. Next: set `--audiowmark <PATH>` or add it to PATH.
cli-status-media_backend = media backend: { $backend }
cli-status-media-eac3 = eac3 decode: { $available }
cli-status-media-containers = containers: { $containers }
cli-status-media-policy = format policy: input { $input_policy }, output { $output_policy }
cli-status-media-policy-input = probe-first (direct wav/flac, decode others)
cli-status-media-policy-output = wav-only
cli-status-value-available = available
cli-status-value-unavailable = unavailable
cli-status-db-mappings = Tag mapping records: { $count }. Next: run `awmkit tag list` to inspect mappings.
cli-status-db-mappings-unavailable = Tag mappings are unavailable. Next: run `awmkit status --doctor --verbose` and check database access. Reason: { $error }
cli-status-db-evidence = Evidence records: { $count }. Next: run `awmkit evidence list --limit 20` to inspect recent rows.
cli-status-db-evidence-unavailable = Evidence records are unavailable. Next: run `awmkit status --doctor --verbose` and check database access. Reason: { $error }

cli-embed-output_single = `--output` supports exactly one input file. Next: pass one input file or remove `--output`.
cli-embed-done = Embed run finished: { $success } succeeded, { $failed } failed. Next: rerun failed files with `--verbose` if needed.
cli-embed-failed = Some files failed to embed. Next: rerun with `--verbose` to inspect diagnostics.
cli-embed-intro-routing-detail = multichannel smart routing enabled (default LFE skip)
cli-embed-intro-parallelism-detail = multichannel route steps use Rayon (max workers: { $workers })
cli-embed-skip-existing = File already contains a watermark and was skipped: { $path }. Next: use a clean source file if re-embedding is required.
cli-embed-precheck-adm-fallback = ADM precheck was unavailable and embedding continued: { $path }. Next: rerun with `--verbose` to inspect fallback reason.
cli-embed-precheck-adm-fallback-detail = ADM precheck fallback on { $path }: { $error }
cli-embed-evidence-store-unavailable-detail = evidence store unavailable: { $error }
cli-embed-evidence-proof-failed-detail = evidence proof failed ({ $input } -> { $output }): { $error }
cli-embed-evidence-insert-failed-detail = evidence insert failed ({ $input } -> { $output }): { $error }
cli-embed-file-ok-snr = Watermark embedded successfully: { $input } -> { $output } (SNR { $snr } dB). Next: run `awmkit detect { $output }` to verify.
cli-embed-file-ok = Watermark embedded successfully: { $input } -> { $output }. Next: run `awmkit detect { $output }` to verify.
cli-embed-snr-unavailable-detail = SNR unavailable for { $input } -> { $output }: { $reason }
cli-embed-file-failed = Embedding failed: { $path }. Next: rerun with `--verbose` to inspect the failure reason.
cli-embed-file-failed-detail = Embed failed on { $path }: { $error }
cli-embed-skipped-count = Files skipped in this run: { $count }. Next: review skipped files before rerunning.
cli-embed-failure-details-title-detail = Failed file diagnostics:
cli-embed-failure-details-item-detail = - { $detail }
cli-embed-failure-details-omitted-detail = - { $count } more failure details omitted
cli-embed-mapping-autosaved = Mapping saved automatically: { $identity } -> { $tag }. Next: run `awmkit tag list` to review mapping records.
cli-embed-mapping-save-failed-detail = mapping save failed: { $error }
cli-embed-mapping-load-failed-detail = mapping load failed: { $error }

cli-detect-done = Detect run finished: { $ok } found, { $miss } miss, { $invalid } invalid. Next: inspect invalid files before any decision.
cli-detect-failed = Some files failed during detect. Next: rerun with `--verbose` to inspect diagnostics.
cli-detect-forensic-warning = This output is not suitable for attribution or forensics. Next: use your formal verification workflow for legal conclusions.
cli-detect-parallelism-detail = multichannel route steps use Rayon (max workers: { $workers })
cli-detect-evidence-store-unavailable-detail = evidence store unavailable: { $error }
cli-detect-file-found = Watermark detected: { $path } (tag { $tag }, identity { $identity }). Next: run `awmkit evidence list --identity { $identity }` to inspect related evidence.
cli-detect-file-found-detail = detect detail for { $path }: clone={ $clone }, score={ $score }, slot_hint={ $slot_hint }, slot_used={ $slot_used }, slot_status={ $slot_status }, scanned={ $slot_scan_count }
cli-detect-file-miss = No watermark detected: { $path }. Next: verify the input source and key slot, then rerun detect.
cli-detect-file-invalid = Watermark is invalid: { $path } ({ $warning }). Next: rerun with `--verbose` and confirm key slot and source integrity.
cli-detect-file-invalid-detail = invalid detect detail for { $path }: error={ $error }, score={ $score }, tag={ $tag }, identity={ $identity }, timestamp={ $timestamp }, slot_unverified={ $slot_unverified }, slot_hint={ $slot_hint }, slot_used={ $slot_used }, slot_status={ $slot_status }, scanned={ $slot_scan_count }
cli-detect-file-error = Detect failed: { $path }. Next: rerun with `--verbose` to inspect the underlying error.
cli-detect-file-error-detail = detect failed on { $path }: { $error }
cli-detect-fallback-detail = fallback trace for { $path }: route={ $route }, reason={ $reason }, outcome={ $outcome }

cli-decode-version = Version: { $version }
cli-decode-timestamp_minutes = Timestamp (minutes): { $minutes }
cli-decode-timestamp_utc = Timestamp (UTC seconds): { $seconds }
cli-decode-key_slot = Key slot: { $key_slot }
cli-decode-tag = Tag: { $tag }
cli-decode-identity = Identity: { $identity }
cli-decode-status_valid = Status: valid

cli-tag-saved = Mapping saved: { $username } -> { $tag }. Next: run `awmkit tag list` to verify.
cli-tag-none = No saved mappings found. Next: run `awmkit tag save --username <name> --tag <tag>` to create one.
cli-tag-removed = Mapping removed: { $username }. Next: run `awmkit tag list` to confirm current mappings.
cli-tag-cleared = All mappings cleared. Next: re-add required mappings with `awmkit tag save`.

cli-evidence-empty = No evidence records found. Next: run `awmkit embed ...` to create new evidence.
cli-evidence-list-row = #{ $id } | { $created_at } | { $identity }/{ $tag } | slot { $slot } | snr { $snr } | sha { $sha } | { $path }
cli-evidence-field-id = ID: { $value }
cli-evidence-field-created_at = Created At: { $value }
cli-evidence-field-file_path = File Path: { $value }
cli-evidence-field-identity = Identity: { $value }
cli-evidence-field-tag = Tag: { $value }
cli-evidence-field-version = Version: { $value }
cli-evidence-field-key_slot = Key Slot: { $value }
cli-evidence-field-timestamp = Timestamp Minutes: { $value }
cli-evidence-field-message_hex = Message Hex: { $value }
cli-evidence-field-sample_rate = Sample Rate: { $value }
cli-evidence-field-channels = Channels: { $value }
cli-evidence-field-sample_count = Sample Count: { $value }
cli-evidence-field-pcm_sha256 = PCM SHA256: { $value }
cli-evidence-field-snr_status = SNR Status: { $value }
cli-evidence-field-snr_db = SNR dB: { $value }
cli-evidence-field-fingerprint_len = Fingerprint Length: { $value }
cli-evidence-field-fp_config_id = Fingerprint Config: { $value }
cli-evidence-not-found = Evidence record was not found: { $id }. Next: run `awmkit evidence list` to locate a valid id.
cli-evidence-removed = Evidence record removed: { $id }. Next: run `awmkit evidence list` to confirm.
cli-evidence-clear-refuse-all = Clear-all was refused. Next: provide at least one filter (`--identity`, `--tag`, or `--key-slot`).
cli-evidence-cleared = Evidence records cleared: { $removed } (identity={ $identity }, tag={ $tag }, key_slot={ $key_slot }). Next: run `awmkit evidence list` to verify remaining rows.
cli-evidence-requires-yes = { $action } was not executed. Next: rerun with `--yes` to confirm.

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
ui-action-run = Run
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
ui-label-auto_mapping = Auto Map
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
ui-danger-clear_cache_desc = This will delete local cache and configuration files.
ui-danger-reset_all = Reset all (delete key)
ui-danger-reset_all_title = Reset All
ui-danger-clear_tags_title = Clear All Mappings
ui-danger-reset_all_desc = This will delete your key, all tag mappings, and all configuration. This cannot be undone.
ui-danger-clear_tags_desc = This will delete all saved tag mappings. This action cannot be undone.
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
