cli-error-quiet_verbose_conflict = 不能同时使用 --quiet 和 --verbose。下一步：删除其中一个参数后重试。
cli-error-key_exists = 当前槽位已存在密钥。下一步：运行 `awmkit key rotate` 替换，或运行 `awmkit key import` 导入新密钥。
cli-error-key_not_found = 当前槽位未配置密钥。下一步：运行 `awmkit init` 或 `awmkit key import`。
cli-error-invalid_key_length = 密钥长度无效（期望 { $expected } 字节，实际 { $actual } 字节）。下一步：提供长度为 { $expected } 字节的密钥。
cli-error-key_store = 密钥库操作失败。下一步：检查密钥后端权限后重试。
cli-error-audiowmark_not_found = audiowmark 不可用。下一步：设置 `--audiowmark <PATH>` 或将二进制加入 PATH。
cli-error-input_not_found = 未找到输入文件：{ $path }。下一步：检查路径后重试。
cli-error-invalid_glob = 通配符模式无效：{ $pattern }。下一步：修正模式，或改为显式传入文件路径。
cli-error-glob = 通配符匹配失败。下一步：检查路径模式并重试。
cli-error-mapping_exists = { $username } 的映射已存在。下一步：使用 `--force` 覆盖，或更换用户名。
cli-error-database = 数据库操作失败。下一步：运行 `awmkit status --doctor --verbose` 并检查数据库权限。
cli-error-config = 配置处理失败。下一步：检查配置路径并重试。
cli-error-io = 文件操作失败。下一步：检查文件路径与权限。
cli-error-hex = 十六进制输入无效。下一步：提供偶数长度的十六进制字符串。
cli-error-audio = 音频处理失败。下一步：确认输入格式受支持，并使用 `--verbose` 重试。
cli-error-json = JSON 处理失败。下一步：使用 `--json` 重试并检查载荷格式。

cli-util-no_input_files = 未提供输入文件。下一步：传入一个或多个输入路径或通配符模式。

cli-init-ok_generated = 已为当前激活槽位生成密钥。下一步：运行 `awmkit key show` 检查密钥信息。
cli-init-ok_stored = 密钥已写入后端（{ $bytes } 字节）。下一步：运行 `awmkit embed ...` 开始嵌入。

cli-key-status_configured = 当前激活槽位已配置密钥。下一步：运行 `awmkit key show` 查看指纹与后端信息。
cli-key-length = 长度：{ $bytes } 字节
cli-key-fingerprint = 指纹（SHA256）：{ $fingerprint }
cli-key-storage = 存储：{ $backend }
cli-key-replaced = 当前激活槽位密钥已替换。下一步：运行 `awmkit key show` 确认指纹。
cli-key-imported = 当前激活槽位密钥已导入。下一步：运行 `awmkit key show` 确认后端与指纹。
cli-key-exported = 当前激活槽位密钥已导出。下一步：请将导出文件安全保存。
cli-key-rotated = 当前激活槽位密钥已轮换。下一步：如有需要，请对近期输出重新执行 `awmkit detect <file>`。
cli-key-slot = 槽位：{ $slot }
cli-key-slot-active = 当前为激活槽位。下一步：如需切换请运行 `awmkit key slot use <slot>`。
cli-key-slot-current_active = 当前激活槽位：{ $slot }
cli-key-imported-slot = 已导入密钥到槽位 { $slot }。下一步：若需激活该槽位，请运行 `awmkit key slot use { $slot }`。
cli-key-exported-slot = 已从槽位 { $slot } 导出密钥。下一步：请将导出文件安全保存。
cli-key-rotated-slot = 槽位 { $slot } 密钥已轮换。下一步：运行 `awmkit key slot list` 检查槽位状态。
cli-key-delete-requires-yes = 未执行密钥删除。下一步：添加 `--yes` 后重试。
cli-key-delete-slot-has-evidence = 槽位 { $slot } 仍有 { $count } 条证据记录。下一步：先清理该槽位证据，或使用 `--force` 重试。
cli-key-deleted-slot = 已删除槽位 { $slot } 密钥。下一步：运行 `awmkit key import --slot { $slot }` 或 `awmkit key rotate --slot { $slot }` 配置新密钥。
cli-key-slot-current = 激活槽位：{ $slot }
cli-key-slot-set = 激活槽位已切换到 { $slot }。下一步：运行 `awmkit key show` 确认该槽位密钥信息。
cli-key-slot-label-set = 已设置槽位 { $slot } 标签为 { $label }。下一步：运行 `awmkit key slot list` 核对标签。
cli-key-slot-label-cleared = 已清除槽位 { $slot } 标签。下一步：如需新标签，请运行 `awmkit key slot label set { $slot } <label>`。
cli-key-conflict-slot-occupied = 检测到槽位冲突：槽位 { $slot } 已有密钥。下一步：先删除该槽位密钥，或改用其他槽位。
cli-key-conflict-slot-has-evidence = 检测到槽位冲突：槽位 { $slot } 仍有 { $count } 条证据记录。下一步：先清理该槽位证据后再继续。
cli-key-conflict-duplicate-fingerprint = 检测到槽位冲突：目标槽位 { $slot } 与槽位 { $conflicts } 指纹重复。下一步：先清理冲突槽位密钥。
cli-key-slot-state-configured = 已配置
cli-key-slot-state-empty = 空
cli-key-slot-active-marker = 激活
cli-key-slot-inactive-marker = 未激活
cli-key-slot-list-row = 槽位 { $slot } [{ $active }]：{ $state }，标签={ $label }，指纹={ $fingerprint }，后端={ $backend }，证据={ $evidence }，最近使用={ $last }
cli-key-error-invalid-slot-input = 槽位参数无效：{ $input }。下一步：请输入整数槽位编号。
cli-key-error-invalid-slot-range = 槽位 { $slot } 超出范围。下一步：请使用 0..={ $max } 范围内的槽位。

cli-status-version = awmkit v{ $version }
cli-status-key_configured = 密钥已配置（{ $bytes } 字节）。下一步：运行 `awmkit key show` 查看指纹信息。
cli-status-key_storage = 密钥后端为 { $backend }。下一步：保持该后端可用以执行密钥操作。
cli-status-key_len_mismatch = 密钥长度与预期不符。下一步：导入或轮换有效密钥。
cli-status-key_not_configured = 密钥未配置。下一步：运行 `awmkit init` 或 `awmkit key import`。
cli-status-audiowmark_available = audiowmark 可用。下一步：运行 `awmkit detect <file>` 验证完整链路。
cli-status-audiowmark_not_responding = audiowmark 无响应。下一步：检查二进制权限与路径。
cli-status-audiowmark_version = audiowmark 版本：{ $version }
cli-status-audiowmark_version_error = audiowmark 版本错误：{ $error }
cli-status-audiowmark_path = audiowmark 路径：{ $path }
cli-status-audiowmark_found = 已找到 audiowmark 二进制。下一步：若仍有运行问题，请运行 `awmkit status --doctor`。
cli-status-audiowmark_not_found = 未找到 audiowmark 二进制。下一步：设置 `--audiowmark <PATH>` 或加入 PATH。
cli-status-media_backend = 媒体后端：{ $backend }
cli-status-media-eac3 = eac3 解码：{ $available }
cli-status-media-containers = 支持容器：{ $containers }
cli-status-media-policy = 格式策略：输入 { $input_policy }，输出 { $output_policy }
cli-status-media-policy-input = 探测优先（WAV/FLAC 直通，其余先解码）
cli-status-media-policy-output = 仅 WAV
cli-status-value-available = 可用
cli-status-value-unavailable = 不可用
cli-status-db-mappings = 标签映射记录数：{ $count }。下一步：运行 `awmkit tag list` 查看映射。
cli-status-db-mappings-unavailable = 标签映射不可用。下一步：运行 `awmkit status --doctor --verbose` 并检查数据库访问。原因：{ $error }
cli-status-db-evidence = 证据记录数：{ $count }。下一步：运行 `awmkit evidence list --limit 20` 查看最近记录。
cli-status-db-evidence-unavailable = 证据记录不可用。下一步：运行 `awmkit status --doctor --verbose` 并检查数据库访问。原因：{ $error }

cli-embed-output_single = `--output` 仅支持一个输入文件。下一步：传入单个输入文件，或移除 `--output`。
cli-embed-done = 嵌入任务完成：{ $success } 成功，{ $failed } 失败。下一步：如需排障，请对失败文件使用 `--verbose` 重试。
cli-embed-failed = 有文件嵌入失败。下一步：使用 `--verbose` 重试查看诊断信息。
cli-embed-intro-routing-detail = 诊断：已启用多声道嵌入路由，默认路由会跳过 LFE。
cli-embed-intro-parallelism-detail = 诊断：多声道路由步骤使用 Rayon 并行（最大 worker：{ $workers }）。
cli-embed-skip-existing = 文件已含水印，已跳过：{ $path }。下一步：如需重新嵌入，请使用干净源文件。
cli-embed-precheck-adm-fallback = ADM 预检不可用，已继续嵌入：{ $path }。下一步：可使用 `--verbose` 查看回退原因。
cli-embed-precheck-adm-fallback-detail = 诊断：{ $path } 触发 ADM 预检回退。error={ $error }
cli-embed-evidence-store-unavailable-detail = 诊断：证据库不可用。error={ $error }
cli-embed-evidence-proof-failed-detail = 诊断：证据指纹构建失败（{ $input } -> { $output }）。error={ $error }
cli-embed-evidence-insert-failed-detail = 诊断：证据记录写入失败（{ $input } -> { $output }）。error={ $error }
cli-embed-file-ok-snr = 水印嵌入成功：{ $input } -> { $output }（SNR { $snr } dB）。下一步：运行 `awmkit detect { $output }` 验证结果。
cli-embed-file-ok = 水印嵌入成功：{ $input } -> { $output }。下一步：运行 `awmkit detect { $output }` 验证结果。
cli-embed-snr-unavailable-detail = 诊断：无法计算 SNR（{ $input } -> { $output }）。reason={ $reason }
cli-embed-file-failed = 水印嵌入失败：{ $path }。下一步：使用 `--verbose` 重试查看失败原因。
cli-embed-file-failed-detail = 诊断：{ $path } 嵌入失败。error={ $error }
cli-embed-skipped-count = 本次跳过文件数：{ $count }。下一步：重试前请先确认这些文件是否需要处理。
cli-embed-failure-details-title-detail = 失败文件诊断汇总：
cli-embed-failure-details-item-detail = - { $detail }
cli-embed-failure-details-omitted-detail = - 其余 { $count } 条失败诊断已省略
cli-embed-mapping-autosaved = 已自动保存映射：{ $identity } -> { $tag }。下一步：运行 `awmkit tag list` 查看映射记录。
cli-embed-mapping-save-failed-detail = 诊断：映射保存失败。error={ $error }
cli-embed-mapping-load-failed-detail = 诊断：映射加载失败。error={ $error }

cli-detect-done = 检测任务完成：{ $ok } 命中，{ $miss } 未命中，{ $invalid } 无效。下一步：先处理无效结果再做结论。
cli-detect-failed = 有文件检测失败。下一步：使用 `--verbose` 重试查看诊断信息。
cli-detect-forensic-warning = 此输出不适用于归属或取证。下一步：法律结论请使用正式校验流程。
cli-detect-parallelism-detail = 诊断：多声道路由步骤使用 Rayon 并行（最大 worker：{ $workers }）。
cli-detect-evidence-store-unavailable-detail = 诊断：证据库不可用。error={ $error }
cli-detect-file-found = 检测到水印：{ $path }（标签 { $tag }，身份 { $identity }）。下一步：运行 `awmkit evidence list --identity { $identity }` 查看关联证据。
cli-detect-file-found-detail = 诊断：{ $path } 检测命中，clone_check={ $clone }，clone_score={ $score }，decode_slot_hint={ $slot_hint }，decode_slot_used={ $slot_used }，slot_status={ $slot_status }，slot_scan_count={ $slot_scan_count }
cli-detect-file-miss = 未检测到水印：{ $path }。下一步：检查输入来源与密钥槽位后重试。
cli-detect-file-invalid = 水印无效：{ $path }（{ $warning }）。下一步：使用 `--verbose` 重试，并确认密钥槽位与源文件完整性。
cli-detect-file-invalid-detail = 诊断：{ $path } 结果无效，error={ $error }，clone_score={ $score }，tag={ $tag }，identity={ $identity }，timestamp={ $timestamp }，slot_unverified={ $slot_unverified }，decode_slot_hint={ $slot_hint }，decode_slot_used={ $slot_used }，slot_status={ $slot_status }，slot_scan_count={ $slot_scan_count }
cli-detect-file-error = 检测失败：{ $path }。下一步：使用 `--verbose` 重试查看底层错误。
cli-detect-file-error-detail = 诊断：{ $path } 检测失败。error={ $error }
cli-detect-fallback-detail = 诊断：{ $path } 回退路径，route={ $route }，reason={ $reason }，outcome={ $outcome }

cli-decode-version = 版本：{ $version }
cli-decode-timestamp_minutes = 时间戳（分钟）：{ $minutes }
cli-decode-timestamp_utc = 时间戳（UTC 秒）：{ $seconds }
cli-decode-key_slot = 密钥槽位：{ $key_slot }
cli-decode-tag = 标签：{ $tag }
cli-decode-identity = 身份：{ $identity }
cli-decode-status_valid = 状态：有效

cli-tag-saved = 映射已保存：{ $username } -> { $tag }。下一步：运行 `awmkit tag list` 核对。
cli-tag-none = 没有已保存映射。下一步：运行 `awmkit tag save --username <name> --tag <tag>` 创建映射。
cli-tag-removed = 映射已移除：{ $username }。下一步：运行 `awmkit tag list` 确认当前映射。
cli-tag-cleared = 已清空所有映射。下一步：使用 `awmkit tag save` 重新添加所需映射。

cli-evidence-empty = 没有证据记录。下一步：运行 `awmkit embed ...` 生成新证据。
cli-evidence-list-row = #{ $id } | { $created_at } | { $identity }/{ $tag } | 槽位 { $slot } | SNR { $snr } | SHA { $sha } | { $path }
cli-evidence-field-id = 编号：{ $value }
cli-evidence-field-created_at = 创建时间：{ $value }
cli-evidence-field-file_path = 文件路径：{ $value }
cli-evidence-field-identity = 身份：{ $value }
cli-evidence-field-tag = 标签：{ $value }
cli-evidence-field-version = 版本：{ $value }
cli-evidence-field-key_slot = 密钥槽位：{ $value }
cli-evidence-field-timestamp = 时间戳（分钟）：{ $value }
cli-evidence-field-message_hex = 消息十六进制：{ $value }
cli-evidence-field-sample_rate = 采样率：{ $value }
cli-evidence-field-channels = 声道数：{ $value }
cli-evidence-field-sample_count = 采样点数：{ $value }
cli-evidence-field-pcm_sha256 = PCM SHA256：{ $value }
cli-evidence-field-snr_status = SNR 状态：{ $value }
cli-evidence-field-snr_db = SNR dB：{ $value }
cli-evidence-field-fingerprint_len = 指纹长度：{ $value }
cli-evidence-field-fp_config_id = 指纹配置：{ $value }
cli-evidence-not-found = 未找到证据记录：{ $id }。下一步：运行 `awmkit evidence list` 查找有效编号。
cli-evidence-removed = 已删除证据记录：{ $id }。下一步：运行 `awmkit evidence list` 确认结果。
cli-evidence-clear-refuse-all = 已拒绝清空全部证据。下一步：至少提供一个过滤条件（`--identity`、`--tag` 或 `--key-slot`）。
cli-evidence-cleared = 已清理证据记录：{ $removed } 条（identity={ $identity }，tag={ $tag }，key_slot={ $key_slot }）。下一步：运行 `awmkit evidence list` 核对剩余记录。
cli-evidence-requires-yes = 未执行 { $action }。下一步：添加 `--yes` 后重试。

ui-window-title = AWMKit GUI
ui-tabs-embed = 水印嵌入
ui-tabs-detect = 水印检测
ui-tabs-status = 系统维护
ui-tabs-tag = 标签管理

ui-page-embed = 水印嵌入
ui-page-detect = 水印检测
ui-page-status = 系统维护
ui-page-tag = 标签管理

ui-action-add_files = 添加文件
ui-action-clear = 清空
ui-action-run = 运行
ui-action-apply = 应用
ui-action-run_embed = 开始嵌入
ui-action-run_detect = 开始检测
ui-action-cancel = 取消
ui-action-refresh = 刷新
ui-action-init_key = 初始化密钥
ui-action-save = 保存
ui-action-remove = 移除
ui-action-clear_mappings = 清空
ui-action-refresh_mappings = 刷新
ui-action-overwrite = 覆盖
ui-action-skip = 跳过
ui-action-browse = 浏览
ui-action-clear_queue = 清空队列
ui-action-clear_all = 全部清除
ui-action-copy_md = 复制 Markdown
ui-action-clear_input = 清空输入
ui-action-clear_output = 清空输出
ui-action-start_process = 开始处理
ui-action-clear_log = 清空日志

ui-label-tag = 标签
ui-label-username_optional = 用户名（可选）
ui-label-username = 用户名
ui-label-use_mapping = 使用映射
ui-label-strength = 强度
ui-label-language = 语言

ui-mapping-select_placeholder = （选择映射）

ui-prompt-save_mapping_title = 保存映射？
ui-prompt-save_mapping_message = 未找到该标签的映射。
ui-prompt-mapping_exists = 映射已存在，点击覆盖替换。
ui-prompt-save_failed = 保存失败：{ $error }

ui-status-ready = 就绪
ui-status-no_input_files = 未选择输入文件。
ui-status-embedding = 正在嵌入...
ui-status-detecting = 正在检测...
ui-status-embed_finished = 嵌入完成。
ui-status-detect_finished = 检测完成。
ui-status-key_initialized = 密钥已初始化。
ui-status-init_failed = 初始化失败：{ $error }
ui-status-mapping_saved = 映射已保存。
ui-status-mapping_not_saved = 映射未保存。
ui-status-mapping_removed = 映射已移除。
ui-status-mappings_cleared = 已清空映射。
ui-status-remove_failed = 移除失败：{ $error }
ui-status-clear_failed = 清空失败：{ $error }
ui-status-save_failed = 保存失败：{ $error }
ui-status-key_error_short = 密钥错误：{ $error }
ui-status-audio_error_short = 音频错误：{ $error }

ui-status-key_configured = 密钥：已配置（{ $bytes } 字节） | { $backend }
ui-status-key_error = 密钥：错误（{ $error }）
ui-status-key_not_configured = 密钥：未配置
ui-status-audiowmark_ok = audiowmark：{ $path }（{ $version }）
ui-status-audiowmark_not_available = audiowmark：不可用（{ $error }）
ui-status-key_not_configured_hint = 密钥未配置，请在“状态 / 初始化”中初始化。

ui-detect-files-title = 文件列表
ui-detect-log-title = 摘要日志
ui-detect-json-title = JSON 详情
ui-detect-summary-title = 检测摘要
ui-detect-detail-title = 结果详情
ui-detect-detail-file = 文件
ui-detect-detail-status = 状态
ui-detect-detail-tag = 标签
ui-detect-detail-identity = 身份
ui-detect-detail-version = 版本
ui-detect-detail-timestamp = 时间戳
ui-detect-detail-pattern = 检测模式
ui-detect-detail-bit_errors = 比特错误数
ui-detect-detail-match_found = 是否匹配
ui-detect-detail-error = 错误
ui-detect-status-ok = 正常
ui-detect-status-found = 已发现
ui-detect-status-not_found = 未发现
ui-detect-status-invalid_hmac = 校验失败
ui-detect-status-error = 错误
ui-detect-summary-waiting = 等待中
ui-detect-summary-found = 已发现
ui-detect-summary-not_found = 未发现
ui-detect-summary-unsupported = 不支持
ui-detect-summary-error = 错误

ui-status-card-key = 密钥
ui-status-card-engine = audiowmark
ui-status-card-actions = 操作
ui-status-card-language = 语言

ui-card-embed_settings = 嵌入设置
ui-card-file_queue = 文件队列
ui-card-result_log = 结果日志
ui-card-input_dir = 输入目录
ui-card-output_dir = 输出目录
ui-card-system_status = 系统状态
ui-card-saved_mappings = 已保存映射
ui-card-add_mapping = 添加映射
ui-card-actions = 操作
ui-card-danger_zone = 危险区域

ui-drop-hint = 拖拽 WAV/FLAC 文件或文件夹到此处
ui-drop-hint-detailed = 拖拽音频文件或目录
ui-drop-hint-sub = 支持混合拖入，目录会递归提取
ui-drop-compact = 拖拽文件或

ui-status-no_input = 尚未添加文件
ui-status-no_output = 尚未选择（默认写回各文件目录）

ui-empty-no_files = 还没有添加文件
ui-empty-no_results = 结果将显示在此处
ui-empty-no_tags = 还没有保存映射
ui-empty-detect = 添加文件以检测水印

ui-status-embed_done = 完成
ui-status-detect_done = 检测完成

ui-danger-title = 危险操作
ui-danger-expand = 展开
ui-danger-collapse = 收起
ui-danger-clear_cache = 清理缓存/配置
ui-danger-clear_cache_title = 清理缓存/配置
ui-danger-clear_cache_desc = 这将删除本地缓存和配置文件。
ui-danger-reset_all = 完全重置（删除密钥）
ui-danger-reset_all_title = 完全重置
ui-danger-clear_tags_title = 清空全部映射
ui-danger-reset_all_desc = 这将删除你的密钥、全部标签映射和所有配置，且不可恢复。
ui-danger-clear_tags_desc = 这将删除所有已保存的标签映射，且不可恢复。
ui-danger-confirm_title = 确认操作
ui-danger-confirm_message_clear = 将删除缓存二进制与配置文件，但不会删除密钥。
ui-danger-confirm_message_reset = 将删除密钥、标签映射与本地配置/缓存，且不可恢复。
ui-danger-confirm_placeholder = 输入 { $code } 确认
ui-danger-confirm_action = 确认
ui-danger-cancel = 取消

ui-status-cache_cleared = 已清理缓存/配置。
ui-status-reset_done = 已完成重置。
ui-status-clear_cache_failed = 清理失败：{ $error }
ui-status-reset_failed = 重置失败：{ $error }

ui-error-tag_required = 必须填写标签
ui-error-username_required = 必须填写用户名
ui-tag-none = 没有已保存的映射
ui-error-key_exists = 密钥已存在
ui-tag-load_failed = 加载失败：{ $error }
ui-log-embed_done = 完成：{ $success } 成功，{ $failed } 失败
ui-log-detect_done = 完成：{ $ok } 成功，{ $missing } 未发现，{ $invalid } 无效

ui-shell-state-ready = 就绪
ui-shell-state-running = 运行中
ui-shell-state-success = 成功
ui-shell-state-error = 错误
ui-shell-working = 处理中...

ui-section-file_queue = 文件队列
ui-section-embed_params = 嵌入参数
ui-section-execution = 执行
ui-section-detect_summary = 检测摘要
ui-section-tag_table = 已保存映射

ui-empty-files-title = 还没有文件
ui-empty-files-description = 添加音频文件后开始嵌入。
ui-empty-execution = 等待执行
ui-empty-detect-summary = 暂无检测结果
ui-empty-detect-files-title = 未选择文件
ui-empty-detect-files-description = 选择音频文件后开始检测。
ui-empty-detect-result-title = 未选择结果
ui-empty-detect-result-description = 请先执行检测并选择一个文件查看详情。
ui-empty-tags-title = 暂无保存映射
ui-empty-tags-description = 保存用户名与标签映射以加快嵌入流程。
ui-label-more_files = 更多文件

ui-status-pending = 等待中
ui-action-expand_json = 展开 JSON
ui-action-suggest = 建议

ui-status-running_init = 正在初始化密钥...
ui-status-running_refresh = 正在刷新状态...
ui-status-running_language = 正在更新语言...
ui-status-running_clear_cache = 正在清理缓存...
ui-status-running_reset = 正在重置全部数据...
ui-status-running_suggest = 正在生成建议标签...
ui-status-running_save = 正在保存映射...
ui-status-running_remove = 正在移除映射...
ui-status-running_clear_tags = 正在清空映射...
ui-status-running_refresh_tags = 正在刷新映射...

ui-status-refresh_done = 状态已刷新。
ui-status-language_updated = 语言已更新。
ui-status-suggest_done = 已生成建议标签。
ui-status-username_required = 必须填写用户名。
ui-status-cancelled = 已取消

ui-danger-confirm_reset_hint = 输入 RESET 以确认。
ui-danger-confirm_awmkit_hint = 输入 AWMKIT 以确认。
ui-danger-confirm_input_hint = 输入 RESET / AWMKIT

ui-status-field-state = 状态
ui-status-field-bytes = 字节数
ui-status-field-backend = 后端
ui-status-field-error = 错误
ui-status-field-path = 路径
ui-status-field-version = 版本
ui-field-state = 状态
ui-field-bytes = 字节数
ui-field-backend = 后端
ui-field-error = 错误
ui-field-path = 路径
ui-field-version = 版本

ui-table-created = 创建时间
ui-status-key_title = 密钥状态

ui-detect-summary-missing = 未发现
ui-detect-summary-invalid = 无效
ui-detect-summary-errors = 错误

ui-toast-copied = 已复制到剪贴板
ui-toast-tag_saved = 标签映射已保存
ui-toast-tag_removed = 标签映射已移除
ui-toast-tags_cleared = 全部映射已清除
ui-toast-cache_cleared = 缓存已清理
ui-toast-reset_done = 重置完成

ui-placeholder-username = 输入用户名
ui-placeholder-tag = 输入或生成标签
ui-placeholder-tag_optional = （可选）

ui-label-auto_mapping = 自动映射

ui-confirm-clear_tags = 确认清空全部映射？

ui-tag-table-username = 用户名
ui-tag-table-tag = 标签
ui-tag-table-created_at = 创建时间

ui-label-success = 成功
ui-label-failed = 失败

ui-state-configured = 已配置
ui-state-not_configured = 未配置
ui-state-error = 错误
ui-state-ok = 正常
ui-state-not_available = 不可用
