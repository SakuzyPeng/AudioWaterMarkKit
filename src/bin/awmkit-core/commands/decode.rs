use crate::error::Result;
use crate::Context;
use awmkit::app::{i18n, KeyStore};
use awmkit::Message;
use clap::Args;
use fluent_bundle::FluentArgs;

#[derive(Args)]
/// Internal struct.
pub struct CmdArgs {
    /// Message hex string (32 chars).
    #[arg(long)]
    pub hex: String,
}

/// Internal helper function.
pub fn run(ctx: &Context, args: &CmdArgs) -> Result<()> {
    let store = KeyStore::new()?;
    let key = store.load()?;
    let bytes = hex::decode(&args.hex)?;

    let decoded = Message::decode(&bytes, &key)?;

    let mut args = FluentArgs::new();
    args.set("version", decoded.version.to_string());
    ctx.out
        .info_user(i18n::tr_args("cli-decode-version", &args));
    let mut args = FluentArgs::new();
    args.set("minutes", decoded.timestamp_minutes.to_string());
    ctx.out
        .info_user(i18n::tr_args("cli-decode-timestamp_minutes", &args));
    let mut args = FluentArgs::new();
    args.set("seconds", decoded.timestamp_utc.to_string());
    ctx.out
        .info_user(i18n::tr_args("cli-decode-timestamp_utc", &args));
    let mut args = FluentArgs::new();
    args.set("key_slot", decoded.key_slot.to_string());
    ctx.out
        .info_user(i18n::tr_args("cli-decode-key_slot", &args));
    let mut args = FluentArgs::new();
    args.set("tag", decoded.tag.to_string());
    ctx.out.info_user(i18n::tr_args("cli-decode-tag", &args));
    let mut args = FluentArgs::new();
    args.set("identity", decoded.identity().to_string());
    ctx.out
        .info_user(i18n::tr_args("cli-decode-identity", &args));
    ctx.out.info_user(i18n::tr("cli-decode-status_valid"));
    Ok(())
}
