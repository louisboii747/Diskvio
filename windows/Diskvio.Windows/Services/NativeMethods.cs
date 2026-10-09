using System.Reflection;
using System.Runtime.InteropServices;
using Microsoft.Win32.SafeHandles;

namespace Diskvio.Windows.Services;

internal static class NativeMethods
{
    private const string LibraryName = "diskvio_ffi";
    private static readonly Lazy<IntPtr> Library = new(LoadLibrary);

    static NativeMethods() => NativeLibrary.SetDllImportResolver(typeof(NativeMethods).Assembly, ResolveLibrary);

    private static IntPtr ResolveLibrary(string name, Assembly assembly, DllImportSearchPath? searchPath) =>
        name == LibraryName ? Library.Value : IntPtr.Zero;

    private static IntPtr LoadLibrary()
    {
        if (RuntimeInformation.ProcessArchitecture != Architecture.X64)
        {
            throw new DiskServiceException("Diskvio's native backend requires a Windows x64 process.");
        }
        var path = Path.Combine(AppContext.BaseDirectory, "diskvio_ffi.dll");
        IntPtr library = IntPtr.Zero;
        try
        {
            library = NativeLibrary.Load(path);
            NativeLibrary.GetExport(library, "diskvio_list_disks_json");
            NativeLibrary.GetExport(library, "diskvio_inventory_json");
            NativeLibrary.GetExport(library, "diskvio_operation_json");
            NativeLibrary.GetExport(library, "diskvio_string_free");
            return library;
        }
        catch (Exception error) when (error is DllNotFoundException or BadImageFormatException or EntryPointNotFoundException)
        {
            if (library != IntPtr.Zero) NativeLibrary.Free(library);
            throw new DiskServiceException(
                $"Could not load the Rust backend at '{path}'. Build Diskvio.Windows as x64 to generate and package diskvio_ffi.dll. {error.Message}", error);
        }
    }

    internal static RustStringHandle ListDisks()
    {
        _ = Library.Value;
        return ListDisksJson();
    }

    internal static RustStringHandle Inventory()
    {
        _ = Library.Value;
        return InventoryJson();
    }

    internal static RustStringHandle Operation(byte[] request)
    {
        if (request.Length is 0 or > 16384) throw new DiskServiceException("The operation request must contain between 1 and 16384 UTF-8 bytes.");
        _ = Library.Value;
        return OperationJson(request, (nuint)request.Length);
    }

    [DllImport(LibraryName, EntryPoint = "diskvio_inventory_json", ExactSpelling = true, CallingConvention = CallingConvention.Cdecl)]
    private static extern RustStringHandle InventoryJson();

    [DllImport(LibraryName, EntryPoint = "diskvio_operation_json", ExactSpelling = true, CallingConvention = CallingConvention.Cdecl)]
    private static extern RustStringHandle OperationJson([In] byte[] request, nuint length);

    [DllImport(LibraryName, EntryPoint = "diskvio_list_disks_json", ExactSpelling = true, CallingConvention = CallingConvention.Cdecl)]
    private static extern RustStringHandle ListDisksJson();

    [DllImport(LibraryName, EntryPoint = "diskvio_string_free", ExactSpelling = true, CallingConvention = CallingConvention.Cdecl)]
    internal static extern void StringFree(IntPtr pointer);
}

internal sealed class RustStringHandle : SafeHandleZeroOrMinusOneIsInvalid
{
    public RustStringHandle() : base(ownsHandle: true) { }

    protected override bool ReleaseHandle()
    {
        NativeMethods.StringFree(handle);
        return true;
    }
}
