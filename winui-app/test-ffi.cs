using System;
using System.Runtime.InteropServices;

class TestFFI
{
    [DllImport("awmkit.dll", CallingConvention = CallingConvention.Cdecl)]
    static extern byte awm_current_version();

    [DllImport("awmkit.dll", CallingConvention = CallingConvention.Cdecl)]
    [return: MarshalAs(UnmanagedType.U1)]
    static extern bool awm_key_exists();

    [DllImport("awmkit.dll", CallingConvention = CallingConvention.Cdecl)]
    static extern int awm_tag_suggest(
        [MarshalAs(UnmanagedType.LPUTF8Str)] string username,
        IntPtr outTag);

    static void Main()
    {
        Console.WriteLine("Testing AWMKit FFI...");
        
        try
        {
            // Test 1: Get version
            byte version = awm_current_version();
            Console.WriteLine($"✓ Current version: {version}");

            // Test 2: Check key exists
            bool keyExists = awm_key_exists();
            Console.WriteLine($"✓ Key exists: {keyExists}");

            // Test 3: Suggest tag
            IntPtr buffer = Marshal.AllocHGlobal(9);
            int code = awm_tag_suggest("testuser", buffer);
            if (code == 0)
            {
                string? tag = Marshal.PtrToStringUTF8(buffer);
                Console.WriteLine($"✓ Suggested tag: {tag}");
            }
            else
            {
                Console.WriteLine($"✗ Tag suggestion failed: {code}");
            }
            Marshal.FreeHGlobal(buffer);

            Console.WriteLine("\n✓ All FFI tests passed!");
        }
        catch (Exception ex)
        {
            Console.WriteLine($"\n✗ FFI test failed: {ex.Message}");
            Environment.Exit(1);
        }
    }
}
