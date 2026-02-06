use crate::error::Result;
use awmkit::app::KeyStore;
use crate::util::parse_tag;
use crate::Context;
use awmkit::{Message, CURRENT_VERSION};
use clap::Args;

#[derive(Args)]
pub struct EncodeArgs {
    /// Tag (1-7 identity or full 8-char tag)
    #[arg(long)]
    pub tag: String,

    /// Protocol version
    #[arg(long, default_value_t = CURRENT_VERSION)]
    pub version: u8,

    /// Timestamp (UTC Unix minutes)
    #[arg(long)]
    pub timestamp: Option<u32>,
}

pub fn run(ctx: &Context, args: &EncodeArgs) -> Result<()> {
    let store = KeyStore::new()?;
    let key = store.load()?;
    let tag = parse_tag(&args.tag)?;

    let message = match args.timestamp {
        Some(ts) => Message::encode_with_timestamp(args.version, &tag, &key, ts)?,
        None => Message::encode(args.version, &tag, &key)?,
    };

    ctx.out.info(hex::encode(message));
    Ok(())
}
