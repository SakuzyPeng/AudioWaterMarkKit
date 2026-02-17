use crate::error::Result;
use crate::util::parse_tag;
use crate::Context;
use awmkit::app::KeyStore;
use awmkit::{Message, CURRENT_VERSION};
use clap::Args;

#[derive(Args)]
/// Internal struct.
pub struct CmdArgs {
    /// Tag (1-7 identity or full 8-char tag).
    #[arg(long)]
    pub tag: String,

    /// Protocol version.
    #[arg(long, default_value_t = CURRENT_VERSION)]
    pub version: u8,

    /// Timestamp (UTC Unix minutes).
    #[arg(long)]
    pub timestamp: Option<u32>,
}

/// Internal helper function.
pub fn run(ctx: &Context, args: &CmdArgs) -> Result<()> {
    let store = KeyStore::new()?;
    let slot = store.active_slot()?;
    let key = store.load_slot(slot)?;
    let tag = parse_tag(&args.tag)?;

    let message = match args.timestamp {
        Some(ts) => Message::encode_with_timestamp_and_slot(args.version, &tag, &key, ts, slot)?,
        None => Message::encode_with_slot(args.version, &tag, &key, slot)?,
    };

    ctx.out.info(hex::encode(message));
    Ok(())
}
