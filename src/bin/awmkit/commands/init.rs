use crate::commands::key;
use crate::error::{CliError, Result};
use crate::Context;
use awmkit::app::{i18n, KeyStore, KEY_LEN};
use fluent_bundle::FluentArgs;

pub fn run(ctx: &Context) -> Result<()> {
    let store = KeyStore::new()?;
    let slot = store.active_slot()?;
    if store.exists_slot(slot) {
        return Err(CliError::Message(i18n::tr("cli-error-key_exists")));
    }
    let slot = key::generate_for_active_slot()?;

    ctx.out.info(i18n::tr("cli-init-ok_generated"));
    let mut args = FluentArgs::new();
    args.set("bytes", KEY_LEN.to_string());
    ctx.out.info(i18n::tr_args("cli-init-ok_stored", &args));
    let mut slot_args = FluentArgs::new();
    slot_args.set("slot", slot.to_string());
    ctx.out.info(i18n::tr_args("cli-key-slot", &slot_args));
    Ok(())
}
