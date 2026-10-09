using System.Runtime.InteropServices;
using System.Text;
using System.Text.Json;
using Diskvio.Windows.Models;

namespace Diskvio.Windows.Services;

public sealed class DiskService : IDiskService
{
    private static readonly UTF8Encoding StrictUtf8 = new(false, true);
    private static readonly JsonSerializerOptions JsonOptions = new() { RespectNullableAnnotations = true };

    // Get-Disk, copying and JSON decoding run on a worker thread. Awaiting the
    // service returns to the view model's captured UI synchronization context.
    public Task<IReadOnlyList<Disk>> ListDisksAsync() => Task.Run(ReadDisks);

    private static IReadOnlyList<Disk> ReadDisks()
    {
        using var response = NativeMethods.ListDisks();
        if (response.IsInvalid)
            throw new DiskServiceException("The Rust backend could not allocate a disk discovery response.");
        try
        {
            var pointer = response.DangerousGetHandle();
            var length = 0;
            const int maximumResponseLength = 16 * 1024 * 1024;
            while (length < maximumResponseLength && Marshal.ReadByte(pointer, length) != 0) length++;
            if (length == maximumResponseLength)
                throw new DiskServiceException("The Rust discovery response exceeded the 16 MiB limit.");
            var bytes = new byte[length];
            Marshal.Copy(pointer, bytes, 0, length);
            return DecodeResponse(StrictUtf8.GetString(bytes));
        }
        catch (DecoderFallbackException error)
        {
            throw new DiskServiceException("The Rust backend returned invalid UTF-8.", error);
        }
        // Dispose frees every valid response once, including on decode failures.
    }

    internal static IReadOnlyList<Disk> DecodeResponse(string json)
    {
        try
        {
            var response = JsonSerializer.Deserialize<DiskResponse>(json, JsonOptions)
                ?? throw new DiskServiceException("The Rust backend returned a null JSON response.");
            if (response.Status == "error")
                throw new DiskServiceException(response.Message ?? "Disk discovery failed without an error message.");
            if (response.Status != "ok" || response.Disks is null || response.Disks.Any(disk => disk is null))
                throw new DiskServiceException("The Rust backend returned an incomplete or unknown discovery response.");
            if (response.Disks.Select(disk => disk.Number).Distinct().Count() != response.Disks.Count)
                throw new DiskServiceException("The Rust backend returned duplicate disk identifiers.");
            return response.Disks;
        }
        catch (JsonException error)
        {
            throw new DiskServiceException("The Rust backend returned an invalid disk discovery JSON contract.", error);
        }
    }
}
