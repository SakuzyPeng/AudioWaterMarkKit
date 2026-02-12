using System;
using System.Collections.Generic;
using System.IO;
using System.Reflection;
using System.Runtime.InteropServices;

namespace AWMKit.Native;

/// <summary>
/// Raw P/Invoke declarations for awmkit native FFI functions.
/// Matches src/ffi.rs signatures exactly.
/// </summary>
internal static class AwmNative
{
    private const string Lib = "awmkit_native.dll";
    private static IntPtr _preloadedHandle = IntPtr.Zero;

    static AwmNative()
    {
        NativeLibrary.SetDllImportResolver(
            typeof(AwmNative).Assembly,
            static (libraryName, assembly, searchPath) => ResolveLibrary(libraryName, assembly));

        _preloadedHandle = ResolveLibrary(Lib, typeof(AwmNative).Assembly);
    }

    internal static bool EnsureLoaded() => _preloadedHandle != IntPtr.Zero;

    private static IntPtr ResolveLibrary(string libraryName, Assembly assembly)
    {
        if (!IsAwmkitLibraryName(libraryName))
        {
            return IntPtr.Zero;
        }

        foreach (var dir in EnumerateNativeSearchDirs())
        {
            var candidate = Path.Combine(dir, Lib);
            if (!File.Exists(candidate))
            {
                continue;
            }

            if (NativeLibrary.TryLoad(candidate, out var handle))
            {
                return handle;
            }
        }

        return IntPtr.Zero;
    }

    private static bool IsAwmkitLibraryName(string libraryName)
    {
        return string.Equals(libraryName, Lib, StringComparison.OrdinalIgnoreCase)
            || string.Equals(libraryName, "awmkit_native", StringComparison.OrdinalIgnoreCase);
    }

    private static IEnumerable<string> EnumerateNativeSearchDirs()
    {
        // Single-file publish: .NET sets this to extraction directories.
        if (AppContext.GetData("NATIVE_DLL_SEARCH_DIRECTORIES") is string raw
            && !string.IsNullOrWhiteSpace(raw))
        {
            foreach (var dir in raw.Split(Path.PathSeparator, StringSplitOptions.RemoveEmptyEntries))
            {
                if (!string.IsNullOrWhiteSpace(dir))
                {
                    yield return dir.Trim();
                }
            }
            yield break;
        }

        // Debug/dev fallback.
        yield return AppContext.BaseDirectory;
    }

    // ── Tag Operations ──

    [DllImport(Lib, EntryPoint = "awm_tag_new", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern int awm_tag_new([MarshalAs(UnmanagedType.LPUTF8Str)] string identity, IntPtr outTag);

    [DllImport(Lib, EntryPoint = "awm_tag_verify", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    [return: MarshalAs(UnmanagedType.U1)]
    internal static extern bool awm_tag_verify([MarshalAs(UnmanagedType.LPUTF8Str)] string tag);

    [DllImport(Lib, EntryPoint = "awm_tag_identity", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern int awm_tag_identity([MarshalAs(UnmanagedType.LPUTF8Str)] string tag, IntPtr outIdentity);

    [DllImport(Lib, EntryPoint = "awm_tag_suggest", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern int awm_tag_suggest([MarshalAs(UnmanagedType.LPUTF8Str)] string username, IntPtr outTag);

    // ── Message Operations ──

    [DllImport(Lib, EntryPoint = "awm_message_encode", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern int awm_message_encode(
        byte version,
        [MarshalAs(UnmanagedType.LPUTF8Str)] string tag,
        IntPtr key,
        nuint keyLen,
        IntPtr outMsg);

    [DllImport(Lib, EntryPoint = "awm_message_encode_with_slot", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern int awm_message_encode_with_slot(
        byte version,
        [MarshalAs(UnmanagedType.LPUTF8Str)] string tag,
        IntPtr key,
        nuint keyLen,
        byte keySlot,
        IntPtr outMsg);

    [DllImport(Lib, EntryPoint = "awm_message_encode_with_timestamp", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern int awm_message_encode_with_timestamp(
        byte version,
        [MarshalAs(UnmanagedType.LPUTF8Str)] string tag,
        IntPtr key,
        nuint keyLen,
        uint timestampMinutes,
        IntPtr outMsg);

    [DllImport(Lib, EntryPoint = "awm_message_decode", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern int awm_message_decode(IntPtr data, IntPtr key, nuint keyLen, IntPtr result);

    [DllImport(Lib, EntryPoint = "awm_message_decode_unverified", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern int awm_message_decode_unverified(IntPtr data, IntPtr result);

    [DllImport(Lib, EntryPoint = "awm_message_verify", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    [return: MarshalAs(UnmanagedType.U1)]
    internal static extern bool awm_message_verify(IntPtr data, IntPtr key, nuint keyLen);

    [DllImport(Lib, EntryPoint = "awm_current_version", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern byte awm_current_version();

    [DllImport(Lib, EntryPoint = "awm_message_length", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern nuint awm_message_length();

    // ── Audio Operations ──

    [DllImport(Lib, EntryPoint = "awm_audio_new", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern IntPtr awm_audio_new();

    [DllImport(Lib, EntryPoint = "awm_audio_new_with_binary", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern IntPtr awm_audio_new_with_binary([MarshalAs(UnmanagedType.LPUTF8Str)] string binaryPath);

    [DllImport(Lib, EntryPoint = "awm_audio_free", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern void awm_audio_free(IntPtr handle);

    [DllImport(Lib, EntryPoint = "awm_audio_set_strength", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern void awm_audio_set_strength(IntPtr handle, byte strength);

    [DllImport(Lib, EntryPoint = "awm_audio_set_key_file", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern void awm_audio_set_key_file(IntPtr handle, [MarshalAs(UnmanagedType.LPUTF8Str)] string keyFile);

    [DllImport(Lib, EntryPoint = "awm_audio_embed", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern int awm_audio_embed(
        IntPtr handle,
        [MarshalAs(UnmanagedType.LPUTF8Str)] string input,
        [MarshalAs(UnmanagedType.LPUTF8Str)] string output,
        IntPtr message);

    [DllImport(Lib, EntryPoint = "awm_audio_embed_multichannel", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern int awm_audio_embed_multichannel(
        IntPtr handle,
        [MarshalAs(UnmanagedType.LPUTF8Str)] string input,
        [MarshalAs(UnmanagedType.LPUTF8Str)] string output,
        IntPtr message,
        AwmChannelLayout layout);

    [DllImport(Lib, EntryPoint = "awm_audio_detect", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern int awm_audio_detect(
        IntPtr handle,
        [MarshalAs(UnmanagedType.LPUTF8Str)] string input,
        IntPtr result);

    [DllImport(Lib, EntryPoint = "awm_audio_detect_multichannel", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern int awm_audio_detect_multichannel(
        IntPtr handle,
        [MarshalAs(UnmanagedType.LPUTF8Str)] string input,
        AwmChannelLayout layout,
        IntPtr result);

    [DllImport(Lib, EntryPoint = "awm_channel_layout_channels", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern uint awm_channel_layout_channels(AwmChannelLayout layout);

    [DllImport(Lib, EntryPoint = "awm_clone_check_for_file", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern int awm_clone_check_for_file(
        [MarshalAs(UnmanagedType.LPUTF8Str)] string input,
        [MarshalAs(UnmanagedType.LPUTF8Str)] string identity,
        byte keySlot,
        IntPtr result);

    [DllImport(Lib, EntryPoint = "awm_evidence_record_file", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern int awm_evidence_record_file(
        [MarshalAs(UnmanagedType.LPUTF8Str)] string filePath,
        IntPtr rawMessage,
        IntPtr key,
        nuint keyLen);

    [DllImport(Lib, EntryPoint = "awm_evidence_record_file_ex", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern int awm_evidence_record_file_ex(
        [MarshalAs(UnmanagedType.LPUTF8Str)] string filePath,
        IntPtr rawMessage,
        IntPtr key,
        nuint keyLen,
        [MarshalAs(UnmanagedType.U1)] bool isForcedEmbed);

    [DllImport(Lib, EntryPoint = "awm_evidence_record_embed_file_ex", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern int awm_evidence_record_embed_file_ex(
        [MarshalAs(UnmanagedType.LPUTF8Str)] string inputPath,
        [MarshalAs(UnmanagedType.LPUTF8Str)] string outputPath,
        IntPtr rawMessage,
        IntPtr key,
        nuint keyLen,
        [MarshalAs(UnmanagedType.U1)] bool isForcedEmbed,
        IntPtr result);

    [DllImport(Lib, EntryPoint = "awm_audio_is_available", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    [return: MarshalAs(UnmanagedType.U1)]
    internal static extern bool awm_audio_is_available(IntPtr handle);

    [DllImport(Lib, EntryPoint = "awm_audio_binary_path", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern int awm_audio_binary_path(IntPtr handle, IntPtr outBuf, nuint outLen);

    [DllImport(Lib, EntryPoint = "awm_audio_media_capabilities", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern int awm_audio_media_capabilities(IntPtr handle, IntPtr result);

    // ── UI Settings (app feature) ──

    [DllImport(Lib, EntryPoint = "awm_ui_language_get", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern int awm_ui_language_get(IntPtr outBuf, nuint outLen, IntPtr outRequiredLen);

    [DllImport(Lib, EntryPoint = "awm_ui_language_set", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern int awm_ui_language_set([MarshalAs(UnmanagedType.LPUTF8Str)] string? langOrNull);

    // ── Key Management (app feature) ──

    [DllImport(Lib, EntryPoint = "awm_key_exists", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    [return: MarshalAs(UnmanagedType.U1)]
    internal static extern bool awm_key_exists();

    [DllImport(Lib, EntryPoint = "awm_key_backend_label", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern int awm_key_backend_label(IntPtr outBuf, nuint outLen);

    [DllImport(Lib, EntryPoint = "awm_key_load", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern int awm_key_load(IntPtr outKey, nuint outKeyCap);

    [DllImport(Lib, EntryPoint = "awm_key_generate_and_save", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern int awm_key_generate_and_save(IntPtr outKey, nuint outKeyCap);

    [DllImport(Lib, EntryPoint = "awm_key_active_slot_get", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern int awm_key_active_slot_get(IntPtr outSlot);

    [DllImport(Lib, EntryPoint = "awm_key_active_slot_set", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern int awm_key_active_slot_set(byte slot);

    [DllImport(Lib, EntryPoint = "awm_key_slot_label_set", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern int awm_key_slot_label_set(byte slot, [MarshalAs(UnmanagedType.LPUTF8Str)] string label);

    [DllImport(Lib, EntryPoint = "awm_key_slot_label_clear", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern int awm_key_slot_label_clear(byte slot);

    [DllImport(Lib, EntryPoint = "awm_key_exists_slot", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    [return: MarshalAs(UnmanagedType.U1)]
    internal static extern bool awm_key_exists_slot(byte slot);

    [DllImport(Lib, EntryPoint = "awm_key_generate_and_save_slot", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern int awm_key_generate_and_save_slot(byte slot, IntPtr outKey, nuint outKeyCap);

    [DllImport(Lib, EntryPoint = "awm_key_delete_slot", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern int awm_key_delete_slot(byte slot, IntPtr outNewActiveSlot);

    [DllImport(Lib, EntryPoint = "awm_key_slot_summaries_json", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern int awm_key_slot_summaries_json(IntPtr outBuf, nuint outLen, IntPtr outRequiredLen);

    [DllImport(Lib, EntryPoint = "awm_key_delete", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern int awm_key_delete();

    // ── Database Operations (app feature) ──

    [DllImport(Lib, EntryPoint = "awm_db_summary", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern int awm_db_summary(IntPtr outTagCount, IntPtr outEvidenceCount);

    [DllImport(Lib, EntryPoint = "awm_db_tag_list_json", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern int awm_db_tag_list_json(uint limit, IntPtr outBuf, nuint outLen, IntPtr outRequiredLen);

    [DllImport(Lib, EntryPoint = "awm_db_tag_lookup", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern int awm_db_tag_lookup(
        [MarshalAs(UnmanagedType.LPUTF8Str)] string username,
        IntPtr outTag,
        nuint outLen,
        IntPtr outRequiredLen);

    [DllImport(Lib, EntryPoint = "awm_db_tag_save_if_absent", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern int awm_db_tag_save_if_absent(
        [MarshalAs(UnmanagedType.LPUTF8Str)] string username,
        [MarshalAs(UnmanagedType.LPUTF8Str)] string tag,
        IntPtr outInserted);

    [DllImport(Lib, EntryPoint = "awm_db_tag_remove_json", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern int awm_db_tag_remove_json(
        [MarshalAs(UnmanagedType.LPUTF8Str)] string usernamesJson,
        IntPtr outDeleted);

    [DllImport(Lib, EntryPoint = "awm_db_evidence_list_json", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern int awm_db_evidence_list_json(uint limit, IntPtr outBuf, nuint outLen, IntPtr outRequiredLen);

    [DllImport(Lib, EntryPoint = "awm_db_evidence_remove_json", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern int awm_db_evidence_remove_json(
        [MarshalAs(UnmanagedType.LPUTF8Str)] string idsJson,
        IntPtr outDeleted);
}
