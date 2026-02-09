using System.Runtime.InteropServices;

namespace AWMKit.Native;

/// <summary>
/// Raw P/Invoke declarations for awmkit.dll FFI functions.
/// Matches src/ffi.rs signatures exactly.
/// </summary>
internal static class AwmNative
{
    private const string Lib = "awmkit.dll";

    // ── Tag Operations ──

    [DllImport(Lib, EntryPoint = "awm_tag_new", CallingConvention = CallingConvention.Cdecl)]
    internal static extern int awm_tag_new([MarshalAs(UnmanagedType.LPUTF8Str)] string identity, IntPtr outTag);

    [DllImport(Lib, EntryPoint = "awm_tag_verify", CallingConvention = CallingConvention.Cdecl)]
    [return: MarshalAs(UnmanagedType.U1)]
    internal static extern bool awm_tag_verify([MarshalAs(UnmanagedType.LPUTF8Str)] string tag);

    [DllImport(Lib, EntryPoint = "awm_tag_identity", CallingConvention = CallingConvention.Cdecl)]
    internal static extern int awm_tag_identity([MarshalAs(UnmanagedType.LPUTF8Str)] string tag, IntPtr outIdentity);

    [DllImport(Lib, EntryPoint = "awm_tag_suggest", CallingConvention = CallingConvention.Cdecl)]
    internal static extern int awm_tag_suggest([MarshalAs(UnmanagedType.LPUTF8Str)] string username, IntPtr outTag);

    // ── Message Operations ──

    [DllImport(Lib, EntryPoint = "awm_message_encode", CallingConvention = CallingConvention.Cdecl)]
    internal static extern int awm_message_encode(
        byte version,
        [MarshalAs(UnmanagedType.LPUTF8Str)] string tag,
        IntPtr key,
        nuint keyLen,
        IntPtr outMsg);

    [DllImport(Lib, EntryPoint = "awm_message_encode_with_timestamp", CallingConvention = CallingConvention.Cdecl)]
    internal static extern int awm_message_encode_with_timestamp(
        byte version,
        [MarshalAs(UnmanagedType.LPUTF8Str)] string tag,
        IntPtr key,
        nuint keyLen,
        uint timestampMinutes,
        IntPtr outMsg);

    [DllImport(Lib, EntryPoint = "awm_message_decode", CallingConvention = CallingConvention.Cdecl)]
    internal static extern int awm_message_decode(IntPtr data, IntPtr key, nuint keyLen, IntPtr result);

    [DllImport(Lib, EntryPoint = "awm_message_verify", CallingConvention = CallingConvention.Cdecl)]
    [return: MarshalAs(UnmanagedType.U1)]
    internal static extern bool awm_message_verify(IntPtr data, IntPtr key, nuint keyLen);

    [DllImport(Lib, EntryPoint = "awm_current_version", CallingConvention = CallingConvention.Cdecl)]
    internal static extern byte awm_current_version();

    [DllImport(Lib, EntryPoint = "awm_message_length", CallingConvention = CallingConvention.Cdecl)]
    internal static extern nuint awm_message_length();

    // ── Audio Operations ──

    [DllImport(Lib, EntryPoint = "awm_audio_new", CallingConvention = CallingConvention.Cdecl)]
    internal static extern IntPtr awm_audio_new();

    [DllImport(Lib, EntryPoint = "awm_audio_new_with_binary", CallingConvention = CallingConvention.Cdecl)]
    internal static extern IntPtr awm_audio_new_with_binary([MarshalAs(UnmanagedType.LPUTF8Str)] string binaryPath);

    [DllImport(Lib, EntryPoint = "awm_audio_free", CallingConvention = CallingConvention.Cdecl)]
    internal static extern void awm_audio_free(IntPtr handle);

    [DllImport(Lib, EntryPoint = "awm_audio_set_strength", CallingConvention = CallingConvention.Cdecl)]
    internal static extern void awm_audio_set_strength(IntPtr handle, byte strength);

    [DllImport(Lib, EntryPoint = "awm_audio_set_key_file", CallingConvention = CallingConvention.Cdecl)]
    internal static extern void awm_audio_set_key_file(IntPtr handle, [MarshalAs(UnmanagedType.LPUTF8Str)] string keyFile);

    [DllImport(Lib, EntryPoint = "awm_audio_embed", CallingConvention = CallingConvention.Cdecl)]
    internal static extern int awm_audio_embed(
        IntPtr handle,
        [MarshalAs(UnmanagedType.LPUTF8Str)] string input,
        [MarshalAs(UnmanagedType.LPUTF8Str)] string output,
        IntPtr message);

    [DllImport(Lib, EntryPoint = "awm_audio_detect", CallingConvention = CallingConvention.Cdecl)]
    internal static extern int awm_audio_detect(
        IntPtr handle,
        [MarshalAs(UnmanagedType.LPUTF8Str)] string input,
        IntPtr result);

    [DllImport(Lib, EntryPoint = "awm_audio_is_available", CallingConvention = CallingConvention.Cdecl)]
    [return: MarshalAs(UnmanagedType.U1)]
    internal static extern bool awm_audio_is_available(IntPtr handle);

    [DllImport(Lib, EntryPoint = "awm_audio_binary_path", CallingConvention = CallingConvention.Cdecl)]
    internal static extern int awm_audio_binary_path(IntPtr handle, IntPtr outBuf, nuint outLen);

    // ── Key Management (app feature) ──

    [DllImport(Lib, EntryPoint = "awm_key_exists", CallingConvention = CallingConvention.Cdecl)]
    [return: MarshalAs(UnmanagedType.U1)]
    internal static extern bool awm_key_exists();

    [DllImport(Lib, EntryPoint = "awm_key_load", CallingConvention = CallingConvention.Cdecl)]
    internal static extern int awm_key_load(IntPtr outKey, nuint outKeyCap);

    [DllImport(Lib, EntryPoint = "awm_key_generate_and_save", CallingConvention = CallingConvention.Cdecl)]
    internal static extern int awm_key_generate_and_save(IntPtr outKey, nuint outKeyCap);

    [DllImport(Lib, EntryPoint = "awm_key_delete", CallingConvention = CallingConvention.Cdecl)]
    internal static extern int awm_key_delete();
}
