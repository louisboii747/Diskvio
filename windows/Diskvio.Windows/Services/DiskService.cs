using System.Runtime.InteropServices;
using System.Text;
using System.Text.Json;
using Diskvio.Windows.Models;

namespace Diskvio.Windows.Services;

public sealed class DiskService : IDiskService
{
    private static readonly UTF8Encoding StrictUtf8 = new(false, true);
    private static readonly JsonSerializerOptions JsonOptions = new() { RespectNullableAnnotations = true };

    public Task<IReadOnlyList<Disk>> ListDisksAsync() => Task.Run(() => DecodeResponse(ReadNative(NativeMethods.ListDisks)));
    public Task<StorageInventory> InventoryAsync() => Task.Run(() => DecodeInventory(ReadNative(NativeMethods.Inventory)));
    public Task<DiskOperationOutcome> PerformAsync(DiskOperationRequest request) => Task.Run(() => DecodeOperation(ReadOperation(request)));
    public Task<OperationValidation> ValidateAsync(DiskOperationRequest request) => Task.Run(() => DecodeValidation(ReadOperation(new { mode = "validate", action = request.Action, identifier = request.Identifier, expected_identity = request.ExpectedIdentity })));
    public Task<OperationCapabilities> SupportedOperationsAsync(string identifier) => Task.Run(() => DecodeCapabilities(ReadOperation(new { mode = "supported_operations", identifier })));

    private static string ReadOperation<T>(T request)
    {
        var bytes = JsonSerializer.SerializeToUtf8Bytes(request, JsonOptions);
        return ReadNative(() => NativeMethods.Operation(bytes));
    }

    private static string ReadNative(Func<RustStringHandle> allocate)
    {
        using var response = allocate();
        if (response.IsInvalid) throw new DiskServiceException("The Rust backend could not allocate a response.");
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
            return StrictUtf8.GetString(bytes);
        }
        catch (DecoderFallbackException error)
        {
            throw new DiskServiceException("The Rust backend returned invalid UTF-8.", error);
        }
    }

    internal static StorageInventory DecodeInventory(string json)
    {
        var inventory = DecodeStorageResponse(json).Inventory ?? throw new DiskServiceException("Missing disk inventory.");
        if (inventory.Disks.Any(disk => disk is null) || inventory.Disks.Select(disk => disk.Number).Distinct().Count() != inventory.Disks.Count)
            throw new DiskServiceException("Invalid or duplicate physical disks in the inventory.");
        if (inventory.Disks.Any(disk => disk.Partitions.Any(partition => partition is null || partition.Volumes.Any(volume => volume is null))) || inventory.UnattachedVolumes.Any(volume => volume is null))
            throw new DiskServiceException("Invalid partitions or volumes in the inventory.");
        var devices = inventory.Disks.SelectMany(disk => new[] { disk.Device }.Concat(disk.Partitions.SelectMany(partition => new[] { partition.Device }.Concat(partition.Volumes.Select(volume => volume.Device)))))
            .Concat(inventory.UnattachedVolumes.Select(volume => volume.Device)).ToList();
        if (devices.Any(device => string.IsNullOrWhiteSpace(device.Identifier)) || devices.Select(device => device.Identifier).Distinct(StringComparer.OrdinalIgnoreCase).Count() != devices.Count)
            throw new DiskServiceException("Missing or ambiguous device identifiers in the inventory.");
        return inventory;
    }

    internal static DiskOperationOutcome DecodeOperation(string json) => DecodeStorageResponse(json).Operation ?? throw new DiskServiceException("Missing operation outcome.");
    internal static OperationCapabilities DecodeCapabilities(string json) => DecodeStorageResponse(json).Capabilities ?? throw new DiskServiceException("Missing operation capabilities.");
    internal static OperationValidation DecodeValidation(string json)
    {
        var validation = DecodeStorageResponse(json).Validation;
        return validation is { Valid: true } ? validation : throw new DiskServiceException("Missing or unsuccessful operation validation.");
    }

    private static StorageResponse DecodeStorageResponse(string json)
    {
        try
        {
            var response = JsonSerializer.Deserialize<StorageResponse>(json, JsonOptions) ?? throw new DiskServiceException("The Rust backend returned a null JSON response.");
            if (response.Status == "error") throw new DiskServiceException(response.Message ?? response.Error?.Message ?? "The storage request failed.", backendError: response.Error);
            if (response.Status != "ok") throw new DiskServiceException("Unknown storage response status.");
            return response;
        }
        catch (JsonException error)
        {
            throw new DiskServiceException("The Rust backend returned an invalid storage JSON contract.", error);
        }
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
