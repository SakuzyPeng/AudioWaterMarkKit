using System;

namespace AWMKit.Data;

/// <summary>
/// Deprecated placeholder. Key slot persistence is managed by Rust KeyStore via FFI.
/// </summary>
[Obsolete("Key slot persistence is managed by Rust KeyStore via FFI. Do not use AppSettingsStore.")]
public sealed class AppSettingsStore
{
    public AppSettingsStore(AppDatabase database)
    {
        throw new NotSupportedException("Use Rust FFI key slot APIs instead of AppSettingsStore.");
    }
}
