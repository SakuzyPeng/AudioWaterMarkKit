use crate::error::{CliError, Result};
use crate::keystore::{generate_key, KeyStore, KEY_LEN};
use crate::Context;

pub fn run(ctx: &Context) -> Result<()> {
    let store = KeyStore::new()?;
    if store.exists() {
        return Err(CliError::Message(
            "key already exists; use `awmkit key rotate` or `awmkit key import`".to_string(),
        ));
    }

    let key = generate_key();
    store.save(&key)?;

    ctx.out.info("[OK] generated key");
    ctx.out
        .info(format!("[OK] stored in keyring ({} bytes)", KEY_LEN));
    Ok(())
}
