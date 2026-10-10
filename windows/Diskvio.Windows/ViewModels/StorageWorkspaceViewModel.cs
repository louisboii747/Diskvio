using System.Collections.ObjectModel;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using Diskvio.Windows.Models;
using Diskvio.Windows.Services;

namespace Diskvio.Windows.ViewModels;

public sealed class StorageWorkspaceViewModel : ObservableObject
{
    private readonly IDiskService _service;
    private StorageNode? _selectedDisk, _selectedNode;
    private bool _busy, _loaded;
    private string _error = "", _result = "", _warnings = "";
    private long _selectionVersion;
    private OperationCapabilities? _capabilities;
    public StorageWorkspaceViewModel(IDiskService service)
    {
        _service = service;
        RefreshCommand = new AsyncRelayCommand(RefreshAsync, () => !IsBusy);
    }
    public ObservableCollection<StorageNode> Disks { get; } = [];
    public ObservableCollection<StorageNode> UnattachedVolumes { get; } = [];
    public IReadOnlyList<StorageNode> Children => SelectedDisk is { } disk
        ? disk.Children.SelectMany(part => new[] { part }.Concat(part.Children)).ToList() : [];
    public ObservableCollection<InspectorProperty> Properties { get; } = [];
    public IAsyncRelayCommand RefreshCommand { get; }
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
    public string PartitionEmptyMessage => SelectedDisk?.Disk?.Partitions.Count == 0 ? "No partitions reported for this disk. Creating partitions is not supported yet." : "Select a partition or volume to inspect it. Right-click or press Shift+F10 for actions.";
    public string SelectionName => SelectedNode?.Name ?? "Select a device";
    public string SelectionSummary => SelectedNode?.Summary ?? "Choose a disk or an unattached volume to inspect.";
    public string DeviceCount => $"{Disks.Count} physical disks";
    public PartitionLayout? Layout => SelectedDisk?.Disk is { } disk ? PartitionLayout.Create(disk) : null;
    public string ActionExplanation => _capabilities?.UnsupportedReason ?? (ActionTarget is null ? "Select a volume to see available management actions." : "Actions require fresh backend authorization. Changes always need confirmation.");
    public string Limitations => _capabilities is { Limitations.Count: > 0 } caps ? string.Join(Environment.NewLine, caps.Limitations) : "Creating and deleting partitions and physical eject are not supported on Windows in this milestone.";
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
        if (node is null || IsBusy) return;
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
            SelectedNode = new(partition.Device, "Partition", disk, partition);
    }
    public void SelectDiskDetails() => SelectedNode = SelectedDisk;
    public async Task RefreshAsync()
    {
        if (IsBusy) return;
        SetBusy(true);
        _error = "";
        try { await LoadInventoryAsync(); }
        catch (Exception e) { ClearInventory(); _error = e.Message; }
        finally { _loaded = true; SetBusy(false); }
        await QueryCapabilitiesAsync();
    }
    private async Task LoadInventoryAsync()
    {
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
        OnPropertyChanged(nameof(Layout));
    }
    private void ClearInventory()
    {
        Disks.Clear(); UnattachedVolumes.Clear(); SelectedDisk = null; SelectedNode = null; _warnings = "";
    }
    // Confirmation owns the busy lease, so selection and refresh cannot race the dialog.
    public async Task ExecuteAsync(DiskAction action, Func<StorageNode, DiskAction, Task<bool>> confirm)
        => await ExecuteWithParametersAsync(action, async (node, a) => await confirm(node, a) ? new OperationInput() : null);

    public async Task ExecuteWithParametersAsync(DiskAction action, Func<StorageNode, DiskAction, Task<OperationInput?>> confirm)
    {
        if (action == DiskAction.Eject || !CanAct(action) || ActionTarget is not { } node) return;
        SetBusy(true);
        _error = ""; _result = "";
        try
        {
            var input = await confirm(node, action);
            if (input is null) return;
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
            // A failed operation can still mean the device changed or disconnected.
            try { await LoadInventoryAsync(); }
            catch (Exception refreshError) { ClearInventory(); _error += Environment.NewLine + refreshError.Message; }
        }
        finally { SetBusy(false); }
        await QueryCapabilitiesAsync();
    }
    private void SetBusy(bool value) { _busy = value; Notify(); RefreshCommand.NotifyCanExecuteChanged(); }
    private void Notify()
    {
        foreach (var name in new[] { nameof(IsBusy), nameof(CanInspect), nameof(IsEmpty), nameof(HasError), nameof(ErrorMessage),
            nameof(HasResult), nameof(ResultMessage), nameof(HasWarnings), nameof(Warnings), nameof(HasSelection), nameof(SelectionName),
            nameof(SelectionSummary), nameof(DeviceCount), nameof(CanMount), nameof(CanUnmount), nameof(CanRename), nameof(CanSetDriveLetter),
            nameof(ActionExplanation), nameof(Limitations), nameof(SelectionBadges), nameof(LabelMaxLength), nameof(HasDiskSelection), nameof(PartitionEmptyMessage) }) OnPropertyChanged(name);
    }
    private void BuildProperties()
    {
        Properties.Clear();
        if (SelectedNode is not { } node) return;
        void Add(string key, object? value) { if (value is not null) Properties.Add(new(key, value.ToString()!)); }
        var d = node.Device;
        Add("Kind", node.Kind); Add("Capacity", d.SizeBytes is { } size ? $"{Models.Disk.FormatCapacity(size)} ({size:N0} bytes)" : "Not reported");
        Add("Used", d.UsedBytes is { } used ? Models.Disk.FormatCapacity(used) : null);
        Add("Available", d.AvailableBytes is { } free ? Models.Disk.FormatCapacity(free) : null);
        Add("File system", node.Filesystem); Add("Volume label", node.Volume?.Label ?? (node.Kind == "Partition" ? node.Partition?.Volumes.FirstOrDefault()?.Label : null));
        Add("Drive letter / mount", node.Location); Add("Partition number", node.Partition?.Number); Add("Partition type", node.Kind != "Physical disk" ? node.Type : null);
        Add("Health", string.IsNullOrEmpty(node.Health) ? null : node.Health); Add("Status", string.IsNullOrEmpty(node.Status) ? null : node.Status);
        Add("Identifier", d.Identifier);
        Add("Stable identifier", d.StableId); Add("Volume UUID", d.VolumeUuid); Add("Partition UUID", d.PartitionUuid);
        Add("Connection", node.Disk?.ConnectionType); Add("Internal", node.Disk?.Internal); Add("Removable", node.Disk?.Removable);
        Add("Partition scheme", node.Disk?.PartitionScheme); Add("Disk UUID", node.Disk?.DiskUuid); Add("MBR signature", node.Disk?.MbrSignature);
        Add("Offset (bytes)", node.Partition?.OffsetBytes); Add("Content type", node.Partition?.ContentType); Add("GPT type", node.Partition?.GptType); Add("MBR type", node.Partition?.MbrType);
        Add("System", d.Safety.System); Add("Boot", d.Safety.Boot); Add("Recovery", d.Safety.Recovery); Add("Hidden", d.Safety.Hidden); Add("Read only", d.Safety.ReadOnly); Add("Offline", d.Safety.Offline); Add("Paging file", d.Safety.PageFile);
        Add("Supported operations", _capabilities is null ? "Checking / unavailable" : _capabilities.Actions.Count == 0 ? "None" : string.Join(", ", _capabilities.Actions.Select(a => a switch { DiskAction.RenameVolume => "Rename label", DiskAction.SetDriveLetter => "Change drive letter", _ => a.ToString() })));
        Add("Action target", ActionTarget?.Kind == "Volume" ? ActionTarget.Name : null);
    }
}

public sealed record OperationInput(string? VolumeLabel = null, string? DriveLetter = null);
