use crate::error::Result;
use awmkit::app::{i18n, KeyStore};
use crate::Context;
use awmkit::Message;
use clap::Args;
use fluent_bundle::FluentArgs;

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

    let mut args = FluentArgs::new();
    args.set("version", decoded.version.to_string());
    ctx.out.info(i18n::tr_args("cli-decode-version", &args));
    let mut args = FluentArgs::new();
    args.set("minutes", decoded.timestamp_minutes.to_string());
    ctx.out
        .info(i18n::tr_args("cli-decode-timestamp_minutes", &args));
    let mut args = FluentArgs::new();
    args.set("seconds", decoded.timestamp_utc.to_string());
    ctx.out
        .info(i18n::tr_args("cli-decode-timestamp_utc", &args));
    let mut args = FluentArgs::new();
    args.set("tag", decoded.tag.to_string());
    ctx.out.info(i18n::tr_args("cli-decode-tag", &args));
    let mut args = FluentArgs::new();
    args.set("identity", decoded.identity().to_string());
    ctx.out.info(i18n::tr_args("cli-decode-identity", &args));
    ctx.out.info(i18n::tr("cli-decode-status_valid"));
    Ok(())
}
