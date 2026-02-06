cli-error-quiet_verbose_conflict = --quiet 和 --verbose 不能同时使用
cli-error-key_exists = 密钥已存在；请使用 `awmkit key rotate` 或 `awmkit key import`
cli-error-key_not_found = 未找到密钥；请运行 `awmkit init` 或 `awmkit key import`
cli-error-invalid_key_length = 密钥长度无效：期望 { $expected } 字节，实际 { $actual }
cli-error-key_store = 密钥库错误：{ $error }
cli-error-audiowmark_not_found = 未找到 audiowmark；请使用 --audiowmark <PATH> 或加入 PATH
cli-error-input_not_found = 未找到输入文件：{ $path }
cli-error-invalid_glob = 通配符模式无效：{ $pattern }
cli-error-glob = 通配符匹配错误：{ $error }
cli-error-mapping_exists = 映射已存在：{ $username }；使用 --force 覆盖

cli-util-no_input_files = 未提供输入文件

cli-init-ok_generated = [OK] 已生成密钥
cli-init-ok_stored = [OK] 已保存至密钥库（{ $bytes } 字节）

cli-key-status_configured = 密钥状态：已配置
cli-key-length = 长度：{ $bytes } 字节
cli-key-fingerprint = 指纹（SHA256）：{ $fingerprint }
cli-key-storage = 存储：{ $backend }
cli-key-replaced = [OK] 已替换密钥
cli-key-imported = [OK] 已导入密钥
cli-key-exported = [OK] 已导出密钥
cli-key-rotated = [OK] 已轮换密钥

cli-status-version = awmkit v{ $version }
cli-status-key_configured = 密钥：已配置（{ $bytes } 字节）
cli-status-key_storage = 密钥存储：{ $backend }
cli-status-key_len_mismatch = 密钥长度与预期不符
cli-status-key_not_configured = 密钥：未配置
cli-status-audiowmark_available = audiowmark：可用
cli-status-audiowmark_not_responding = audiowmark：无响应
cli-status-audiowmark_version = audiowmark 版本：{ $version }
cli-status-audiowmark_version_error = audiowmark 版本错误：{ $error }
cli-status-audiowmark_path = audiowmark 路径：{ $path }
cli-status-audiowmark_found = audiowmark：已找到
cli-status-audiowmark_not_found = audiowmark：未找到

cli-embed-output_single = --output 仅支持单个输入文件
cli-embed-done = 完成：{ $success } 成功，{ $failed } 失败
cli-embed-failed = 有文件处理失败

cli-detect-done = 完成：{ $ok } 成功，{ $miss } 未发现，{ $invalid } 无效
cli-detect-failed = 有文件处理失败

cli-decode-version = 版本：{ $version }
cli-decode-timestamp_minutes = 时间戳（分钟）：{ $minutes }
cli-decode-timestamp_utc = 时间戳（UTC 秒）：{ $seconds }
cli-decode-tag = 标签：{ $tag }
cli-decode-identity = 身份：{ $identity }
cli-decode-status_valid = 状态：有效

cli-tag-saved = 已保存：{ $username } -> { $tag }
cli-tag-none = 没有已保存的映射
cli-tag-removed = 已移除：{ $username }
cli-tag-cleared = 已清空所有映射

ui-window-title = AWMKit GUI
ui-tabs-embed = 嵌入
ui-tabs-detect = 检测
ui-tabs-status = 状态 / 初始化
ui-tabs-tag = 标签管理

ui-page-embed = 嵌入
ui-page-detect = 检测
ui-page-status = 状态 / 初始化
ui-page-tag = 标签管理

ui-action-add_files = 添加文件
ui-action-clear = 清空
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
ui-detect-detail-tag = Tag
ui-detect-detail-identity = 身份
ui-detect-detail-version = 版本
ui-detect-detail-timestamp = 时间戳
ui-detect-detail-pattern = Pattern
ui-detect-detail-bit_errors = Bit errors
ui-detect-detail-match_found = Match found
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

ui-danger-title = 危险操作
ui-danger-expand = 展开
ui-danger-collapse = 收起
ui-danger-clear_cache = 清理缓存/配置
ui-danger-reset_all = 完全重置（删除密钥）
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

ui-confirm-clear_tags = 确认清空全部映射？
ui-placeholder-username = 输入用户名
ui-placeholder-tag = 输入或生成标签

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
