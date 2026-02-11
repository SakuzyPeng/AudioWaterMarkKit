# Glossary

[中文](../zh-CN/TERMS.md)

- `Identity`: 7-character identity string used to derive the 8-character tag.
- `Tag`: 8-character identifier (`Identity + checksum`) encoded into watermark messages.
- `Message`: 16-byte payload (Version + Time/Slot + TagPacked + HMAC).
- `Key Slot`: Key slot id (`0..31`) used to separate active keys.
- `Key ID`: Short key fingerprint (SHA-256 prefix) for human-readable key distinction.
- `Evidence`: Post-embed record (PCM SHA256 + Chromaprint + metadata).
- `Clone Check`: Detect-time evidence matching result (`exact/likely/suspect/unavailable`).
- `Slot Status`: Decode slot diagnostic (`matched/recovered/mismatch/missing_key/ambiguous`).
