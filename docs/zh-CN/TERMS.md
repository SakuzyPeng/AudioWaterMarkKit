# 术语表

[English](../en-US/TERMS.md)

- `Identity`：7 字符身份串，用于派生 8 字符 Tag。
- `Tag`：8 字符标识（`Identity + 校验位`），用于编码水印消息。
- `Message`：16 字节水印消息（Version + Time/Slot + TagPacked + HMAC）。
- `Key Slot`：密钥槽位（`0..31`），用于区分不同活动密钥。
- `Key ID`：密钥指纹短串（SHA-256 前缀），用于可视化区分密钥。
- `Evidence`：嵌入后记录的证据行（PCM SHA256 + Chromaprint + 元数据）。
- `Clone Check`：检测后与证据库比对的结果（`exact/likely/suspect/unavailable`）。
- `Slot Status`：解码槽位诊断（`matched/recovered/mismatch/missing_key/ambiguous`）。
