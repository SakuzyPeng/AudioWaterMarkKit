//! 媒体解码后端

#[cfg(feature = "ffmpeg-decode")]
mod ffmpeg_decode;

#[cfg(feature = "ffmpeg-decode")]
pub(crate) use ffmpeg_decode::{decode_media_to_pcm_i32, media_capabilities};
