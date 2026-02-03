use crate::error::Result;
use crate::keystore::KeyStore;
use crate::Context;
use awmkit::Message;
use clap::Args;

#[derive(Args)]
pub struct DecodeArgs {
    /// Message hex string (32 chars)
    #[arg(long)]
    pub hex: String,
}

pub fn run(ctx: &Context, args: &DecodeArgs) -> Result<()> {
    let store = KeyStore::new()?;
    let key = store.load()?;
    let bytes = hex::decode(&args.hex)?;

    let decoded = Message::decode(&bytes, &key)?;

    ctx.out.info(format!("Version: {}", decoded.version));
    ctx.out
        .info(format!("Timestamp (minutes): {}", decoded.timestamp_minutes));
    ctx.out
        .info(format!("Timestamp (UTC seconds): {}", decoded.timestamp_utc));
    ctx.out.info(format!("Tag: {}", decoded.tag));
    ctx.out.info(format!("Identity: {}", decoded.identity()));
    ctx.out.info("Status: valid");
    Ok(())
}
