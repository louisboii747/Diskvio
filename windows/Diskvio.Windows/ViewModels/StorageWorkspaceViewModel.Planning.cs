using System.Globalization;
using Diskvio.Windows.Models;
using Diskvio.Windows.Services;

namespace Diskvio.Windows.ViewModels;

public sealed partial class StorageWorkspaceViewModel
{
    private DiskPlanningDiscovery? _planningDiscovery;
    private IReadOnlyList<StorageNode> _regionNodes = [];
    private PartitionPlanningResult? _partitionPreview;
    private PhysicalDiskIdentity? _designatedIdentity;
    private long _planningVersion;
    private bool _refreshPending, _createEditor, _deleteEditor;
    private string _requestedSize = "", _sizeUnit = "MiB", _requestedStart = "", _planningStatus = "Inspect layout to begin";
    private CancellationTokenSource? _debounce;

    public void Dispose()
    {
        ClearPreview();
        GC.SuppressFinalize(this);
    }

    public DiskPlanningDiscovery? PlanningDiscovery => _planningDiscovery;
    public PartitionPlanningResult? PartitionPreview => _partitionPreview;
    public bool HasValidatedPreview => _partitionPreview?.Validation.Valid == true;
    public bool IsCreateEditor => _createEditor;
    public bool IsDeleteEditor => _deleteEditor;
    public bool IsPlanningEditor => _createEditor || _deleteEditor;
    public bool IsTestDiskDesignated => _designatedIdentity is not null && _designatedIdentity == _planningDiscovery?.PhysicalIdentity;
    public bool CanDesignateTestDisk => !IsBusy && !HasWarnings && _planningDiscovery is { Validation.Valid: true, PhysicalIdentity: not null, LayoutSnapshot: not null, Geometry: not null };
    public bool CanBeginCreatePreview => !IsBusy && SelectedNode?.Region is { Validation.Valid: true, MaximumAllocatableBytes: > 0 } && CanDesignateTestDisk;
    public bool CanBeginDeletePreview => !IsBusy && SelectedDeletion is not null;
    public bool CanRequestPartitionPreview => !IsBusy && IsPlanningEditor && IsTestDiskDesignated && !HasWarnings &&
        (_createEditor ? CanBeginCreatePreview : SelectedDeletion is { Eligible: true, Validation.Valid: true });
    public string PlanningStatus => _planningStatus;
    public string TestDiskStatus => IsTestDiskDesignated ? "Designated for this session only. Backend safety checks still apply." : "No test media designated. Explicit designation of this exact disk is required.";
    public string PlanningDiskIdentity => _planningDiscovery?.PhysicalIdentity is { } identity
        ? $"{_planningDiscovery.Name} · {_planningDiscovery.Identifier}\nVPD {identity.UniqueId} (format {identity.UniqueIdFormat})\nSerial {identity.SerialNumber}" : "Persistent disk identity not available";
    public string UnallocatedSummary => _planningDiscovery is { Geometry: not null } discovery && discovery.LayoutSnapshot is not null
        ? $"{Disk.FormatCapacity(discovery.Regions.Aggregate(0ul, (total, r) => checked(total + r.CapacityBytes)))} unallocated · {discovery.Regions.Count} verified regions"
        : "Unallocated capacity unknown. Inspect the backend layout.";
    public string DiskOverview => SelectedDisk?.Disk is { } disk
        ? $"{disk.Device.Name} · Disk {disk.Number}\nManufacturer: {disk.Manufacturer ?? "Not reported"} · Model: {disk.Model ?? "Not reported"}\n{(disk.Device.SizeBytes is { } size ? $"{Disk.FormatCapacity(size)} ({size:N0} bytes)" : "Capacity unknown")} · {disk.ConnectionType ?? "Bus unknown"} · {disk.PartitionScheme}\n{(disk.Device.Safety.Offline is { } offline ? offline ? "Offline" : "Online" : "Online status unknown")} · {(disk.Device.Safety.ReadOnly is { } readOnly ? readOnly ? "Read-only" : "Writable media" : "Read-only status unknown")} · {disk.Partitions.Count} partitions\n{UnallocatedSummary}\nPlanning: {(_planningDiscovery is { } found ? found.Validation.Valid ? "Backend eligibility verified; explicit test-media designation required" : PartitionPlanInput.Status(found.Validation) : "Eligibility has not been checked")}" : "Select a physical disk.";
    public string RegionSummary => SelectedNode?.Region is { } region
        ? $"Region [{region.Identity.StartOffsetBytes:N0}, {region.Identity.EndOffsetBytes:N0}) bytes\nGeometric capacity: {region.CapacityBytes:N0} bytes\nAligned capacity: {region.AlignedCapacityBytes:N0} bytes\nSafely allocatable: {region.MaximumAllocatableBytes:N0} bytes\nRequired alignment / minimum size: {region.RequiredAlignmentBytes:N0} bytes\nFirst aligned start: {region.FirstUsableAlignedOffsetBytes?.ToString("N0", CultureInfo.CurrentCulture) ?? "Unavailable"} bytes\n{PartitionPlanInput.Issues(region.Validation)}"
        : "Select a backend-reported unallocated region.";
    public string DeletionSummary => SelectedDeletion is { } deletion
        ? $"Partition {deletion.Partition.Number} · {deletion.Partition.PartitionId}\nSize: {deletion.Partition.SizeBytes:N0} bytes\nGPT type: {deletion.Partition.GptType}\nAssociated volumes: {string.Join(", ", deletion.Partition.Volumes.Select(v => v.Identifier).DefaultIfEmpty("None reported"))}\nMount points: {string.Join(", ", deletion.Partition.Volumes.SelectMany(v => v.MountPoints).DefaultIfEmpty("None reported"))}\nSafety: {(deletion.Eligible && deletion.Validation.Valid ? "Eligible for preview with test-media designation" : "Protected or unverified")}\n{PartitionPlanInput.Issues(deletion.Validation)}"
        : "Partition planning metadata is unavailable. Inspect the layout.";
    public string DeletionWarning => SelectedDeletion?.DataLossWarning ?? "Deleting this partition would permanently remove its partition entry and could make its data inaccessible. This preview changes no storage.";
    public string PreviewDetails => _partitionPreview is { Validation.Valid: true, CreatePreview: { } p }
        ? $"Proposed GPT Microsoft basic-data partition\nStart: {p.StartOffsetBytes:N0} bytes\nEnd (exclusive): {p.EndOffsetBytes:N0} bytes\nSize: {p.AllocatedSizeBytes:N0} bytes\nAlignment: {p.RequiredAlignmentBytes:N0} bytes\nRemaining geometric capacity: {p.RemainingRegionCapacityBytes:N0} bytes\nRemaining aligned capacity: {p.RemainingAllocatableCapacityBytes:N0} bytes\nValidated by Rust. Nothing has been applied."
        : _partitionPreview is { Validation.Valid: true, DeletePreview: { Eligible: true } deletion }
            ? $"Partition {deletion.Partition.Number} would be replaced by {deletion.Partition.SizeBytes:N0} bytes of unallocated space.\nValidated by Rust. Nothing has been applied.\n{deletion.DataLossWarning}"
            : "No validated proposal. Current disk contents are unchanged.";
    public PartitionLayout? PreviewLayout => HasValidatedPreview && Layout is { } layout ? layout.Preview(_partitionPreview!) : null;
    public string? SelectedMapId => SelectedNode?.Region is { } region ? PartitionPlanInput.RegionId(region) : SelectedNode?.Partition?.Device.Identifier;
    private DeletePartitionPreview? SelectedDeletion => SelectedNode?.Partition is { } partition
        ? _planningDiscovery?.Partitions.FirstOrDefault(p => string.Equals(p.Partition.PartitionId?.Trim('{', '}'), partition.Device.PartitionUuid?.Trim('{', '}'), StringComparison.OrdinalIgnoreCase) && p.Partition.PartitionId is not null && p.Partition.Number == partition.Number && p.Partition.OffsetBytes == partition.OffsetBytes && p.Partition.SizeBytes == partition.Device.SizeBytes)
        : null;
    private IEnumerable<StorageNode> RegionNodes => _regionNodes;
    private void UpdateRegionNodes() => _regionNodes = _planningDiscovery?.Regions.Select(region => new StorageNode(
        new StorageDevice { Identifier = PartitionPlanInput.RegionId(region), Name = "Unallocated space", SizeBytes = region.CapacityBytes },
        "Unallocated region", SelectedDisk?.Disk, Region: region)).ToList() ?? [];

    public string RequestedSize { get => _requestedSize; set { if (SetProperty(ref _requestedSize, value)) PlanningInputChanged(); } }
    public string SizeUnit { get => _sizeUnit; set { if (SetProperty(ref _sizeUnit, value)) PlanningInputChanged(); } }
    public string RequestedStart { get => _requestedStart; set { if (SetProperty(ref _requestedStart, value)) PlanningInputChanged(); } }

    public void SelectRegion(AllocatableRegion region)
    {
        var node = RegionNodes.FirstOrDefault(n => n.Region == region);
        if (node is not null) SelectedNode = node;
    }

    public void DesignateTestDisk(PhysicalDiskIdentity confirmedIdentity)
    {
        if (!CanDesignateTestDisk || confirmedIdentity != _planningDiscovery?.PhysicalIdentity) return;
        _designatedIdentity = confirmedIdentity;
        NotifyPlanning();
        SchedulePreview();
    }

    public void RevokeTestDiskDesignation()
    {
        _designatedIdentity = null;
        ClearPreview();
        NotifyPlanning();
    }

    public void BeginCreatePreview()
    {
        if (!CanBeginCreatePreview) return;
        _createEditor = true; _deleteEditor = false;
        ResetPartitionPreview();
    }

    public void BeginDeletePreview()
    {
        if (!CanBeginDeletePreview) return;
        ClearPreview();
        _createEditor = false; _deleteEditor = true;
        _planningStatus = SelectedDeletion is { Eligible: true, Validation.Valid: true }
            ? "Deletion preview only. Designate test media, then request Preview."
            : "Protected or rejected target. See the backend restrictions below.";
        NotifyPlanning();
    }

    public void ResetPartitionPreview()
    {
        ClearPreview();
        _requestedSize = "1"; _sizeUnit = "MiB"; _requestedStart = "";
        _planningStatus = "Preview only. Enter an exact size; leave start blank to use the backend's first aligned position.";
        NotifyPlanning();
        SchedulePreview();
    }

    public void CancelPartitionPreview()
    {
        InvalidatePlanning(false);
        _planningStatus = "Preview cancelled. Nothing was changed.";
        NotifyPlanning();
    }

    private void ClearPreview()
    {
        _planningVersion++;
        _debounce?.Cancel();
        _debounce?.Dispose();
        _debounce = null;
        _partitionPreview = null;
    }

    private void InvalidatePlanning(bool clearDiscovery)
    {
        ClearPreview();
        _createEditor = false; _deleteEditor = false;
        if (clearDiscovery)
        {
            _planningDiscovery = null; _designatedIdentity = null; _regionNodes = [];
            if (_selectedNode?.Region is not null) SelectedNode = SelectedDisk;
            _partitionPlanningSummary = "Read the backend layout to inspect allocation boundaries and partition eligibility.";
            _planningStatus = "Inspect layout to begin";
        }
        else
        {
            _partitionPlanningSummary = _planningDiscovery is { } discovery ? "Preview only. Partition changes cannot execute.\n" + PartitionPlanInput.Issues(discovery.Validation) : "Read the backend layout to inspect allocation boundaries and partition eligibility.";
            _planningStatus = _planningDiscovery is null ? "Inspect layout to begin" : "Select a target to plan. Previous proposals were discarded.";
        }
        NotifyPlanning();
    }

    private void PlanningInputChanged()
    {
        ClearPreview();
        _planningStatus = "Inputs changed. Awaiting a fresh backend preview.";
        NotifyPlanning();
        SchedulePreview();
    }

    private void SchedulePreview()
    {
        if (!_createEditor || !IsTestDiskDesignated) return;
        _debounce?.Cancel();
        _debounce?.Dispose();
        _debounce = new();
        _ = DebouncedPreviewAsync(_debounce.Token);
    }

    private async Task DebouncedPreviewAsync(CancellationToken token)
    {
        try
        {
            await Task.Delay(400, token);
            while (IsBusy) await Task.Delay(100, token);
            if (!token.IsCancellationRequested) await PreviewPartitionAsync();
        }
        catch (OperationCanceledException) { }
    }

    public async Task InspectPartitionPlanningAsync()
    {
        if (!CanInspectPartitionPlanning || SelectedDisk is not { } disk || _service is not IPartitionPlanningService service) return;
        InvalidatePlanning(true);
        var version = _planningVersion;
        _error = ""; _result = "";
        SetBusy(true);
        _planningStatus = "Reading authoritative geometry and safety metadata…";
        NotifyPlanning();
        try
        {
            var response = await service.PlanPartitionAsync(new DiscoverPartitionLayoutRequest { Identifier = disk.Device.Identifier });
            if (version != _planningVersion) return;
            CheckContract(response);
            if (response.Discovery is { } discovery && discovery.Identifier != disk.Device.Identifier)
                throw new DiskServiceException("The backend returned another disk. Refresh and inspect again.");
            if (response.Discovery is { Geometry: not null, LayoutSnapshot: not null } verified && !MatchesInventory(verified, disk.Disk!))
                throw new DiskServiceException("The partition inventory changed during inspection. Refresh disks, then inspect the layout again.", backendError: new() { Code = "changed_partition_layout", Message = "Inventory and planning layouts differ." });
            _planningDiscovery = response.Discovery;
            UpdateRegionNodes();
            _planningStatus = response.Validation.Valid ? "Layout verified. Select unallocated space or a partition to plan." : PartitionPlanInput.Status(response.Validation);
            _partitionPlanningSummary = "Preview only. Partition changes cannot execute.\n" + PartitionPlanInput.Issues(response.Validation);
            if (_planningDiscovery is { } found)
                _partitionPlanningSummary += string.Concat(found.Regions.Select(r => $"\nAligned capacity: {r.AlignedCapacityBytes:N0} bytes. Eligible capacity: {r.MaximumAllocatableBytes:N0} bytes."));
            BuildProperties();
            NotifyPlanning();
        }
        catch (Exception error)
        {
            if (version == _planningVersion) SetPlanningFailure(error);
        }
        finally { SetBusy(false); if (_refreshPending) await RefreshAsync(); }
    }

    public PartitionPlanRequest BuildPartitionRequest()
    {
        if (!IsTestDiskDesignated || _planningDiscovery is not { Validation.Valid: true, PhysicalIdentity: { } identity, LayoutSnapshot: { } snapshot } discovery)
            throw new InvalidOperationException("Designate this exact eligible disk as expendable external test media for this session.");
        if (_createEditor && SelectedNode?.Region is { Validation.Valid: true, MaximumAllocatableBytes: > 0 } region && region.ParentIdentity == identity)
        {
            if (!PartitionPlanInput.TryBytes(RequestedSize, SizeUnit, out var size) || size == 0)
                throw new InvalidOperationException("Enter a positive exact size in MiB or GiB. No rounding is performed.");
            ulong? start = null;
            if (RequestedStart.Length > 0)
            {
                if (!PartitionPlanInput.TryBytes(RequestedStart, "bytes", out var offset))
                    throw new InvalidOperationException("Enter an exact nonnegative starting offset in bytes, or leave it blank.");
                start = offset;
            }
            return new CreatePartitionPlanRequest
            {
                Identifier = discovery.Identifier, ExpectedIdentity = identity, ExpectedLayout = snapshot,
                TestDisk = new() { Identity = _designatedIdentity! }, Region = region.Identity, RequestedSizeBytes = size,
                RequestedStartOffsetBytes = start, RequiredAlignmentBytes = region.RequiredAlignmentBytes,
                PartitionTable = region.PartitionTable, ProposedGptType = PartitionPlanInput.BasicDataType
            };
        }
        if (_deleteEditor && SelectedDeletion is { Eligible: true, Validation.Valid: true } deletion)
            return new DeletePartitionPlanRequest
            {
                Identifier = discovery.Identifier, ExpectedIdentity = identity, ExpectedLayout = snapshot,
                TestDisk = new() { Identity = _designatedIdentity! }, Partition = PartitionPlanInput.Target(deletion.Partition)
            };
        throw new InvalidOperationException("The selected target is protected, unsupported or no longer in this layout. Inspect the layout again.");
    }

    public async Task PreviewPartitionAsync()
    {
        if (!CanRequestPartitionPreview || _service is not IPartitionPlanningService service) return;
        ClearPreview();
        var version = _planningVersion;
        PartitionPlanRequest request;
        try { request = BuildPartitionRequest(); }
        catch (InvalidOperationException error) { _planningStatus = error.Message; NotifyPlanning(); return; }
        SetBusy(true);
        _planningStatus = "Validating preview against the current physical disk…";
        NotifyPlanning();
        try
        {
            var response = await service.PlanPartitionAsync(request);
            if (version != _planningVersion) return;
            CheckContract(response);
            if (!response.Validation.Valid)
            {
                _planningStatus = PartitionPlanInput.Status(response.Validation);
                _partitionPlanningSummary = PartitionPlanInput.Issues(response.Validation);
                if (response.Validation.Issues.Any(i => i.Code is "stale_device_identity" or "changed_partition_layout" or "device_disappeared" or "ambiguous_identity" or "discovery_failed" or "permission_denied" or "protected_target" or "missing_metadata"))
                {
                    _planningDiscovery = null; _designatedIdentity = null; _regionNodes = [];
                    if (SelectedNode?.Region is not null)
                    {
                        SelectedNode = SelectedDisk;
                        _planningStatus = PartitionPlanInput.Status(response.Validation);
                        _partitionPlanningSummary = PartitionPlanInput.Issues(response.Validation);
                    }
                }
                NotifyPlanning();
                return;
            }
            if (response.Discovery is not { Validation.Valid: true } current || current.PhysicalIdentity != request.ExpectedIdentity || current.LayoutSnapshot != request.ExpectedLayout)
                throw new DiskServiceException("Preview identity or layout does not match the selected disk. Refresh and inspect again.");
            if (request is CreatePartitionPlanRequest create && (response.CreatePreview is not { } p || p.ParentIdentity != create.ExpectedIdentity || p.ExpectedLayout != create.ExpectedLayout || p.Region != create.Region || p.AllocatedSizeBytes != create.RequestedSizeBytes || p.RequiredAlignmentBytes != create.RequiredAlignmentBytes || p.ProposedGptType != create.ProposedGptType || p.PartitionTable != create.PartitionTable || (create.RequestedStartOffsetBytes is { } offset && p.StartOffsetBytes != offset)))
                throw new DiskServiceException("The backend returned a preview for different creation parameters. Inspect the layout again.");
            if (request is DeletePartitionPlanRequest delete && (response.DeletePreview is not { Eligible: true, Validation.Valid: true } d || d.ParentIdentity != delete.ExpectedIdentity || PartitionPlanInput.Target(d.Partition) != delete.Partition))
                throw new DiskServiceException("The backend returned a different or ineligible deletion target. Inspect the layout again.");
            if (Layout?.Preview(response) is null)
                throw new DiskServiceException("The proposal does not match the displayed physical layout. Refresh disks and inspect again.");
            _partitionPreview = response;
            _planningStatus = "Technically valid preview. Disk contents are unchanged.";
            _partitionPlanningSummary = "Rust revalidated persistent identity, layout, geometry and safety. This preview does not reserve or modify the disk.";
            NotifyPlanning();
        }
        catch (Exception error)
        {
            if (version == _planningVersion)
            {
                _partitionPreview = null; _planningDiscovery = null; _designatedIdentity = null; _regionNodes = [];
                if (SelectedNode?.Region is not null) SelectedNode = SelectedDisk;
                SetPlanningFailure(error);
            }
        }
        finally { SetBusy(false); if (_refreshPending) await RefreshAsync(); }
    }

    private static void CheckContract(PartitionPlanningResult result)
    {
        if (result.ApiVersion != 1 || !result.PreviewOnly || result.ExecutionSupported || result.Validation.Valid != (result.Validation.Issues.Count == 0))
            throw new DiskServiceException("Invalid preview-only backend contract. No proposal is available.");
    }

    private static bool MatchesInventory(DiskPlanningDiscovery discovery, StorageDisk disk) =>
        disk.Device.SizeBytes == discovery.Geometry?.DiskSizeBytes && disk.Partitions.Count == discovery.Partitions.Count &&
        discovery.Partitions.All(d => disk.Partitions.Any(p => d.Partition.PartitionId is not null &&
            string.Equals(d.Partition.PartitionId.Trim('{', '}'), p.Device.PartitionUuid?.Trim('{', '}'), StringComparison.OrdinalIgnoreCase) &&
            d.Partition.Number == p.Number && d.Partition.OffsetBytes == p.OffsetBytes && d.Partition.SizeBytes == p.Device.SizeBytes &&
            string.Equals(d.Partition.GptType?.Trim('{', '}'), p.GptType?.Trim('{', '}'), StringComparison.OrdinalIgnoreCase)));

    private void SetPlanningFailure(Exception error)
    {
        _planningStatus = error is DiskServiceException { BackendError: { } issue }
            ? PartitionPlanInput.Status(new() { Valid = false, Issues = [issue] }) : "Backend discovery failed";
        _partitionPlanningSummary = error.Message;
        NotifyPlanning();
    }

    private void AddPlanningProperties()
    {
        if (SelectedNode?.Region is { } region)
        {
            Properties.Add(new("Region bounds (bytes)", $"[{region.Identity.StartOffsetBytes:N0}, {region.Identity.EndOffsetBytes:N0})"));
            Properties.Add(new("Safely allocatable (bytes)", region.MaximumAllocatableBytes.ToString("N0", CultureInfo.CurrentCulture)));
            Properties.Add(new("Alignment (bytes)", region.RequiredAlignmentBytes.ToString("N0", CultureInfo.CurrentCulture)));
            Properties.Add(new("Eligibility", region.Validation.Valid ? "Eligible geometry; explicit test designation required" : PartitionPlanInput.Issues(region.Validation)));
        }
        if (SelectedDeletion is { } deletion) Properties.Add(new("Deletion eligibility", deletion.Eligible ? "Preview only; test designation required" : PartitionPlanInput.Issues(deletion.Validation)));
    }

    private void NotifyPlanning()
    {
        foreach (var name in new[] { nameof(PlanningDiscovery), nameof(PartitionPreview), nameof(HasValidatedPreview), nameof(IsCreateEditor), nameof(IsDeleteEditor), nameof(IsPlanningEditor), nameof(IsTestDiskDesignated), nameof(CanDesignateTestDisk), nameof(CanBeginCreatePreview), nameof(CanBeginDeletePreview), nameof(CanRequestPartitionPreview), nameof(PlanningStatus), nameof(TestDiskStatus), nameof(PlanningDiskIdentity), nameof(UnallocatedSummary), nameof(DiskOverview), nameof(RegionSummary), nameof(DeletionSummary), nameof(DeletionWarning), nameof(PreviewDetails), nameof(PreviewLayout), nameof(SelectedMapId), nameof(RequestedSize), nameof(SizeUnit), nameof(RequestedStart), nameof(Layout), nameof(Children), nameof(PartitionPlanningSummary) }) OnPropertyChanged(name);
    }
}
