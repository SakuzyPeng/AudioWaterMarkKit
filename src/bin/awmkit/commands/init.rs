use crate::error::{CliError, Result};
use crate::Context;
use awmkit::app::{generate_key, i18n, KeyStore, KEY_LEN};
use fluent_bundle::FluentArgs;

pub fn run(ctx: &Context) -> Result<()> {
    let store = KeyStore::new()?;
    if store.exists() {
        return Err(CliError::Message(i18n::tr("cli-error-key_exists")));
    }

    let key = generate_key();
    store.save(&key)?;

    ctx.out.info(i18n::tr("cli-init-ok_generated"));
    let mut args = FluentArgs::new();
    args.set("bytes", KEY_LEN.to_string());
    ctx.out.info(i18n::tr_args("cli-init-ok_stored", &args));
    Ok(())
}
