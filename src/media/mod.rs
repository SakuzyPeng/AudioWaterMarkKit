//! 媒体解码后端

#[cfg(feature = "multichannel")]
pub mod adm_bwav;
#[cfg(feature = "multichannel")]
pub mod adm_embed;
#[cfg(feature = "multichannel")]
pub mod adm_routing;

#[cfg(feature = "ffmpeg-decode")]
mod ffmpeg_decode;

#[cfg(feature = "ffmpeg-decode")]
pub use ffmpeg_decode::{decode_media_to_pcm_i32, decode_media_to_wav_pipe, media_capabilities};
