using System.Collections.ObjectModel;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using Diskvio.Windows.Models;
using Diskvio.Windows.Services;

namespace Diskvio.Windows.ViewModels;

public sealed partial class StorageWorkspaceViewModel : ObservableObject, IDisposable
{
    private readonly IDiskService _service;
    private StorageNode? _selectedDisk, _selectedNode;
    private bool _busy, _loaded;
    private string _error = "", _result = "", _warnings = "";
    private long _selectionVersion;
    private OperationCapabilities? _capabilities;
    private string _partitionPlanningSummary = "Read the backend layout to inspect allocation boundaries and partition eligibility.";
    public StorageWorkspaceViewModel(IDiskService service)
    {
        _service = service;
        RefreshCommand = new AsyncRelayCommand(RefreshAsync, () => !IsBusy);
        InspectPartitionPlanningCommand = new AsyncRelayCommand(InspectPartitionPlanningAsync, () => CanInspectPartitionPlanning);
    }
    public ObservableCollection<StorageNode> Disks { get; } = [];
    public ObservableCollection<StorageNode> UnattachedVolumes { get; } = [];
    public IReadOnlyList<StorageNode> Children => SelectedDisk is { } disk
        ? SortedPartitions(disk.Children).SelectMany(part => new[] { part }.Concat(part.Children)).Concat(RegionNodes).ToList() : [];
    public ObservableCollection<InspectorProperty> Properties { get; } = [];
    public IAsyncRelayCommand RefreshCommand { get; }
    public IAsyncRelayCommand InspectPartitionPlanningCommand { get; }
    public bool CanInspectPartitionPlanning => !IsBusy && SelectedDisk is not null && _service is IPartitionPlanningService;
    public string PartitionPlanningSummary => _partitionPlanningSummary;
    public bool IsBusy => _busy;
    public bool CanInspect => !IsBusy;
    public bool IsEmpty => _loaded && !IsBusy && !HasError && Disks.Count == 0;
    public bool HasError => _error.Length > 0;
    public string ErrorMessage => _error;
    public bool HasResult => _result.Length > 0;
    public string ResultMessage => _result;
    public bool HasWarnings => _warnings.Length > 0;
    public string Warnings => _warnings;
    public bool HasSelection => SelectedNode is not null;
    public bool HasDiskSelection => SelectedDisk is not null;
    public string PartitionEmptyMessage => SelectedDisk is null ? "Select a physical disk to inspect its layout." : "Select a partition, volume or backend-reported unallocated region. Right-click or press Shift+F10 for actions.";
    public string SelectionName => SelectedNode?.Name ?? "Select a device";
    public string SelectionSummary => SelectedNode?.Summary ?? "Choose a disk or an unattached volume to inspect.";
    public string DeviceCount => $"{Disks.Count} physical disks";
    public PartitionLayout? Layout => SelectedDisk?.Disk is { } disk ? PartitionLayout.Create(disk, PlanningDiscovery) : null;
    public string ActionExplanation => SelectedNode?.Region is { } region ? region.Validation.Valid ? "Unallocated region from Rust. Partition configuration is preview only and requires explicit test-media designation." : PartitionPlanInput.Issues(region.Validation) : _capabilities?.UnsupportedReason ?? (ActionTarget is null ? "Select a volume to see available management actions." : "Actions require fresh backend authorization. Changes always need confirmation.");
    public string Limitations => "Partition creation and deletion are preview only. Formatting, resizing and physical eject are unavailable." + (_capabilities is { Limitations.Count: > 0 } caps ? Environment.NewLine + string.Join(Environment.NewLine, caps.Limitations) : "");
    public string SelectionBadges => SelectedNode?.Badges ?? "";
    public int LabelMaxLength => _capabilities?.LabelMaxLength ?? 11;
    public StorageNode? ActionTarget => SelectedNode is { Kind: "Partition", Children.Count: 1 } part ? part.Children[0] : SelectedNode;
    public StorageNode? SelectedDisk
    {
        get => _selectedDisk;
        set
        {
            if (SetProperty(ref _selectedDisk, value))
            {
                InvalidatePlanning(true);
                OnPropertyChanged(nameof(Layout));
                OnPropertyChanged(nameof(Children));
                SelectedNode = value;
            }
        }
    }
    public StorageNode? SelectedNode
    {
        get => _selectedNode;
        set
        {
            if (!SetProperty(ref _selectedNode, value)) return;
            _selectionVersion++;
            _capabilities = null;
            InvalidatePlanning(false);
            BuildProperties();
            Notify();
        }
    }
    public bool CanMount => CanAct(DiskAction.Mount);
    public bool CanUnmount => CanAct(DiskAction.Unmount);
    public bool CanRename => CanAct(DiskAction.RenameVolume);
    public bool CanSetDriveLetter => CanAct(DiskAction.SetDriveLetter);
    private bool CanAct(DiskAction action) => !IsBusy && !HasWarnings &&
        ActionTarget is { Kind: "Volume", Disk.Internal: false } node &&
        node.Disk.ConnectionType?.Equals("USB", StringComparison.OrdinalIgnoreCase) == true &&
        node.Device.Safety is { System: not true, Boot: not true, Recovery: not true, PageFile: not true, ReadOnly: not true, Offline: not true, Hidden: not true } &&
        (action is not (DiskAction.RenameVolume or DiskAction.SetDriveLetter) ||
            node.Device.Safety is { System: false, Boot: false, Recovery: false, PageFile: false, ReadOnly: false, Offline: false, Hidden: false }) &&
        !string.IsNullOrWhiteSpace(node.Device.IdentityToken) && node.Device.Actions.Contains(action) &&
        _capabilities is { DeviceKind: "volume" } caps && caps.Identifier == node.Device.Identifier &&
        caps.IdentityToken == node.Device.IdentityToken && caps.Actions.Contains(action);

    public async Task QueryCapabilitiesAsync()
    {
        var node = ActionTarget;
        var version = _selectionVersion;
        if (node is null || node.Region is not null || IsBusy) return;
        try
        {
            var caps = await _service.SupportedOperationsAsync(node.Device.Identifier);
            if (version != _selectionVersion) return;
            _capabilities = caps;
            BuildProperties();
            Notify();
        }
        catch (Exception e)
        {
            if (version != _selectionVersion) return;
            _capabilities = null;
            _error = e.Message;
            Notify();
        }
    }

    public void SelectPartition(StoragePartition partition)
    {
        if (SelectedDisk?.Disk is { } disk)
            SelectedNode = Children.FirstOrDefault(n => n.Kind == "Partition" && n.Device.Identifier == partition.Device.Identifier) ?? new(partition.Device, "Partition", disk, partition);
    }
    public void SelectDiskDetails() => SelectedNode = SelectedDisk;
    public async Task RefreshAsync()
    {
        InvalidatePlanning(true);
        if (IsBusy) { _refreshPending = true; return; }
        _refreshPending = false;
        SetBusy(true);
        _error = "";
        try { await LoadInventoryAsync(); }
        catch (Exception e) { ClearInventory(); _error = e.Message; }
        finally { _loaded = true; SetBusy(false); if (_refreshPending) await RefreshAsync(); }
        await QueryCapabilitiesAsync();
    }
    private async Task LoadInventoryAsync()
    {
        InvalidatePlanning(true);
        var diskId = SelectedDisk?.Device.Identifier;
        var diskStable = SelectedDisk?.Device.StableId;
        var nodeId = SelectedNode?.Device.Identifier;
        var nodeIdentity = SelectedNode?.Device.IdentityToken;
        var inventory = await _service.InventoryAsync();
        Disks.Clear();
        UnattachedVolumes.Clear();
        foreach (var disk in inventory.Disks.OrderBy(d => d.Number)) Disks.Add(new(disk.Device, "Physical disk", disk));
        foreach (var volume in inventory.UnattachedVolumes) UnattachedVolumes.Add(new(volume.Device, "Volume", Volume: volume));
        _warnings = string.Join(Environment.NewLine, inventory.Warnings);
        SelectedDisk = Disks.FirstOrDefault(d => diskStable is not null ? d.Device.StableId == diskStable : d.Device.Identifier == diskId) ?? Disks.FirstOrDefault();
        var all = Disks.SelectMany(d => new[] { d }.Concat(d.Children.SelectMany(p => new[] { p }.Concat(p.Children)))).Concat(UnattachedVolumes);
        var restored = all.FirstOrDefault(n => (nodeIdentity is not null ? n.Device.IdentityToken == nodeIdentity : n.Device.Identifier == nodeId) &&
            (n.Disk is null || n.Disk.Device.StableId == diskStable)) ?? SelectedDisk;
        if (restored is { Disk: null }) SelectedDisk = null;
        SelectedNode = restored;
        if (nodeId is not null && !all.Any(n => n.Device.Identifier == nodeId || nodeIdentity is not null && n.Device.IdentityToken == nodeIdentity))
            _deviceNotice = "The previously selected device is no longer available. The device list has been updated.";
        _lastUpdated = DateTimeOffset.Now;
        NotifyBrowser();
        OnPropertyChanged(nameof(Layout));
    }
    private void ClearInventory()
    {
        Disks.Clear(); UnattachedVolumes.Clear(); SelectedDisk = null; SelectedNode = null; _warnings = "";
        NotifyBrowser();
    }
    public async Task ExecuteAsync(DiskAction action, Func<StorageNode, DiskAction, Task<bool>> confirm)
        => await ExecuteWithParametersAsync(action, async (node, a) => await confirm(node, a) ? new OperationInput() : null);

    public async Task ExecuteWithParametersAsync(DiskAction action, Func<StorageNode, DiskAction, Task<OperationInput?>> confirm)
    {
        if (action == DiskAction.Eject || !CanAct(action) || ActionTarget is not { } node) return;
        InvalidatePlanning(true);
        SetBusy(true);
        _activity = $"Awaiting confirmation for {node.Name}…";
        Notify();
        _error = ""; _result = "";
        try
        {
            var input = await confirm(node, action);
            if (input is null) return;
            _activity = $"{action switch { DiskAction.RenameVolume => "Renaming", DiskAction.SetDriveLetter => "Changing drive letter for", DiskAction.Mount => "Mounting", _ => "Unmounting" }} {node.Name}…";
            Notify();
            var request = new DiskOperationRequest { Action = action, Identifier = node.Device.Identifier, ExpectedIdentity = node.Device.IdentityToken!, VolumeLabel = input.VolumeLabel, DriveLetter = input.DriveLetter,
                ExpectedMountPoints = action is DiskAction.RenameVolume or DiskAction.SetDriveLetter ? [.. node.Device.MountPoints] : null };
            var caps = await _service.SupportedOperationsAsync(request.Identifier);
            if (caps.Identifier != request.Identifier || caps.IdentityToken != request.ExpectedIdentity ||
                caps.DeviceKind != "volume" || !caps.Actions.Contains(action))
                throw new DiskServiceException("The device or its supported operations changed. Refresh and select it again.");
            var validation = await _service.ValidateAsync(request);
            if (!validation.Valid || validation.Action != action || validation.Identifier != request.Identifier || validation.ExpectedIdentity != request.ExpectedIdentity)
                throw new DiskServiceException("The backend could not validate the selected device.");
            var outcome = await _service.PerformAsync(request);
            if (outcome.Action != action || outcome.Identifier != request.Identifier)
                throw new DiskServiceException("The backend returned an unexpected operation result. Refresh to check the device state.");
            _result = outcome.Message;
            await LoadInventoryAsync();
        }
        catch (Exception e)
        {
            _error = e.Message;
            try { await LoadInventoryAsync(); }
            catch (Exception refreshError) { ClearInventory(); _error += Environment.NewLine + refreshError.Message; }
        }
        finally { _activity = ""; SetBusy(false); if (_refreshPending) await RefreshAsync(); }
        await QueryCapabilitiesAsync();
    }
    private void SetBusy(bool value) { _busy = value; Notify(); NotifyPlanning(); RefreshCommand.NotifyCanExecuteChanged(); InspectPartitionPlanningCommand.NotifyCanExecuteChanged(); }
    private void Notify()
    {
        foreach (var name in new[] { nameof(IsBusy), nameof(CanInspect), nameof(IsEmpty), nameof(HasError), nameof(ErrorMessage),
            nameof(HasResult), nameof(ResultMessage), nameof(HasWarnings), nameof(Warnings), nameof(HasSelection), nameof(SelectionName),
            nameof(SelectionSummary), nameof(DeviceCount), nameof(CanMount), nameof(CanUnmount), nameof(CanRename), nameof(CanSetDriveLetter),
            nameof(ActionExplanation), nameof(Limitations), nameof(SelectionBadges), nameof(LabelMaxLength), nameof(HasDiskSelection), nameof(PartitionEmptyMessage), nameof(CanInspectPartitionPlanning), nameof(PartitionPlanningSummary),
            nameof(DiskName), nameof(DiskSubtitle), nameof(DiskModel), nameof(DiskCapacity), nameof(DiskConnection), nameof(DiskScheme), nameof(DiskStatus), nameof(InspectorTitle), nameof(SelectedIdentifier), nameof(HasVolumeActions),
            nameof(Activity), nameof(BrowserSummary), nameof(LastUpdated), nameof(HasDeviceNotice), nameof(DeviceNotice), nameof(IsInitialLoading), nameof(HasUnattachedSelection), nameof(BrowserSelection) }) OnPropertyChanged(name);
        InspectPartitionPlanningCommand.NotifyCanExecuteChanged();
    }
    private void BuildProperties()
    {
        Properties.Clear();
        if (SelectedNode is not { } node) { OnPropertyChanged(nameof(PropertySections)); return; }
        void Add(string key, object? value) { if (value is not null) Properties.Add(new(key, value.ToString()!)); }
        var d = node.Device;
        Add("Kind", node.Kind); Add("Capacity", d.SizeBytes is { } size ? $"{Models.Disk.FormatCapacity(size)} ({size:N0} bytes)" : "Not reported");
        Add("Used", node.UsedBytes is { } used ? $"{Models.Disk.FormatCapacity(used)} ({used:N0} bytes)" : "Not reported");
        Add("Available", node.AvailableBytes is { } free ? $"{Models.Disk.FormatCapacity(free)} ({free:N0} bytes)" : "Not reported");
        Add("File system", node.Filesystem); Add("Volume label", node.Volume?.Label ?? (node.Kind == "Partition" ? node.Partition?.Volumes.FirstOrDefault()?.Label : null));
        Add("Drive letter / mount", node.Location); Add("Partition number", node.Partition?.Number); Add("Partition type", node.Kind != "Physical disk" ? node.Type : null);
        Add("Health", string.IsNullOrEmpty(node.Health) ? null : node.Health); Add("Status", string.IsNullOrEmpty(node.Status) ? null : node.Status);
        Add("Identifier", d.Identifier);
        Add("Stable identifier", d.StableId); Add("Volume UUID", d.VolumeUuid); Add("Partition UUID", d.PartitionUuid);
        Add("Connection", node.Disk?.ConnectionType); Add("Internal", node.Disk?.Internal); Add("Removable", node.Disk?.Removable);
        Add("Disk number", node.Disk?.Number); Add("Manufacturer", node.Disk?.Manufacturer ?? "Not reported"); Add("Model", node.Disk?.Model ?? "Not reported");
        Add("Partitions", node.Disk?.Partitions.Count); Add("Online", node.Disk?.Device.Safety.Offline is { } offline ? (!offline).ToString() : "Unknown");
        Add("Partition scheme", node.Disk?.PartitionScheme); Add("Disk UUID", node.Disk?.DiskUuid); Add("MBR signature", node.Disk?.MbrSignature);
        Add("Offset (bytes)", node.Partition?.OffsetBytes); Add("Content type", node.Partition?.ContentType); Add("GPT type", node.Partition?.GptType); Add("MBR type", node.Partition?.MbrType);
        Add("System", d.Safety.System); Add("Boot", d.Safety.Boot); Add("Recovery", d.Safety.Recovery); Add("Hidden", d.Safety.Hidden); Add("Read only", d.Safety.ReadOnly); Add("Offline", d.Safety.Offline); Add("Paging file", d.Safety.PageFile);
        Add("Supported operations", _capabilities is null ? "Checking / unavailable" : _capabilities.Actions.Count == 0 ? "None" : string.Join(", ", _capabilities.Actions.Select(a => a switch { DiskAction.RenameVolume => "Rename label", DiskAction.SetDriveLetter => "Change drive letter", _ => a.ToString() })));
        Add("Action target", ActionTarget?.Kind == "Volume" ? ActionTarget.Name : null);
        AddPlanningProperties();
        OnPropertyChanged(nameof(PropertySections));
    }
}

public sealed record OperationInput(string? VolumeLabel = null, string? DriveLetter = null);
