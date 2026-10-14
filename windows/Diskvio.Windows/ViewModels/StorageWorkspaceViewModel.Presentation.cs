using Diskvio.Windows.Models;

namespace Diskvio.Windows.ViewModels;

public sealed partial class StorageWorkspaceViewModel
{
    private string _searchText = "", _deviceFilter = "All devices", _partitionSort = "Physical order", _deviceNotice = "", _activity = "";
    private DateTimeOffset? _lastUpdated;
    public string SearchText { get => _searchText; set { if (SetProperty(ref _searchText, value)) NotifyBrowser(); } }
    public string DeviceFilter { get => _deviceFilter; set { if (SetProperty(ref _deviceFilter, value)) NotifyBrowser(); } }
    public string PartitionSort { get => _partitionSort; set { if (SetProperty(ref _partitionSort, value)) OnPropertyChanged(nameof(Children)); } }
    public IReadOnlyList<DeviceGroup> DeviceGroups => Disks.Where(MatchesDevice)
        .SelectMany(disk => SearchText.Trim().Length == 0 ? new[] { disk } : new[] { disk }.Concat(disk.Children.SelectMany(p => new[] { p }.Concat(p.Children))).Where(MatchesSearchSelf))
        .GroupBy(n => n.GroupName)
        .OrderBy(g => g.Key switch { "Internal" => 0, "External" => 1, "Removable" => 2, _ => 3 })
        .Select(g => new DeviceGroup(g.Key, g)).ToList();
    public IReadOnlyList<StorageNode> VisibleUnattachedVolumes => UnattachedVolumes.Where(MatchesSearch).ToList();
    public bool HasNoMatches => Disks.Count > 0 && DeviceGroups.Count == 0;
    public bool IsInitialLoading => IsBusy && !_loaded && Disks.Count == 0;
    public bool HasUnattachedSelection => SelectedNode is { Disk: null };
    public StorageNode? BrowserSelection => SearchText.Trim().Length == 0 ? SelectedDisk : SelectedNode;
    public bool HasUnattachedVolumes => VisibleUnattachedVolumes.Count > 0;
    public string BrowserSummary => IsBusy ? Activity : $"{Disks.Count} physical {(Disks.Count == 1 ? "disk" : "disks")}";
    public string Activity => string.IsNullOrEmpty(_activity) ? "Refreshing device information…" : _activity;
    public string LastUpdated => _lastUpdated is { } updated ? $"Updated {updated.LocalDateTime:t}" : "Storage explorer";
    public bool HasDeviceNotice => _deviceNotice.Length > 0;
    public string DeviceNotice => _deviceNotice;
    public string DiskName => SelectedDisk?.Name ?? SelectionName;
    public string DiskSubtitle => SelectedDisk is { } disk ? $"Disk {disk.Disk?.Number} · {disk.Device.Identifier}" : SelectionSummary;
    public string DiskModel => SelectedDisk?.Disk?.Model ?? SelectedDisk?.Disk?.Manufacturer ?? "Model not reported";
    public string DiskCapacity => SelectedDisk?.Capacity ?? "Not reported";
    public string DiskConnection => SelectedDisk?.Disk?.ConnectionType ?? "Not reported";
    public string DiskScheme => SelectedDisk?.Disk?.PartitionScheme.ToUpperInvariant() ?? "Not reported";
    public string DiskStatus => SelectedDisk?.ReportedStatus ?? "Status not reported";
    public string InspectorTitle => SelectedNode?.Kind ?? "Information";
    public string SelectedIdentifier => SelectedNode?.Device.Identifier ?? "";
    public bool HasVolumeActions => CanMount || CanUnmount || CanRename || CanSetDriveLetter;
    public IReadOnlyList<InspectorSection> PropertySections => Properties.GroupBy(p => PropertySection(p.Label))
        .OrderBy(g => g.Key switch { "General" => 0, "Storage" => 1, "Connection" => 2, "Protection & actions" => 3, _ => 4 })
        .Select(g => new InspectorSection(g.Key, g.ToList())).ToList();
    public void DismissResultMessage() { _result = ""; Notify(); }
    public void DismissDeviceNotice() { _deviceNotice = ""; Notify(); }
    public void SelectBrowserNode(StorageNode node)
    {
        SelectedDisk = node.Kind == "Physical disk" ? node : Disks.FirstOrDefault(d => d.Device.Identifier == node.Disk?.Device.Identifier);
        SelectedNode = node.Kind == "Physical disk" ? SelectedDisk : Children.FirstOrDefault(n => n.Device.Identifier == node.Device.Identifier) ?? node;
    }

    private IEnumerable<StorageNode> SortedPartitions(IEnumerable<StorageNode> partitions) => PartitionSort switch
    {
        "Name" => partitions.OrderBy(n => n.Name, StringComparer.CurrentCultureIgnoreCase),
        "Largest first" => partitions.OrderByDescending(n => n.Device.SizeBytes),
        _ => partitions
    };
    private bool MatchesDevice(StorageNode node) => (DeviceFilter switch
    {
        "Internal drives" => node.Disk?.Internal == true,
        "External drives" => node.Disk?.Internal == false,
        "Removable devices" => node.Disk?.Removable == true,
        _ => true
    }) && MatchesSearch(node);
    private bool MatchesSearch(StorageNode node)
    {
        return new[] { node }.Concat(node.Children.SelectMany(p => new[] { p }.Concat(p.Children))).Any(MatchesSearchSelf);
    }
    private bool MatchesSearchSelf(StorageNode node)
    {
        var query = SearchText.Trim();
        return query.Length == 0 || new[] { node.Name, node.Device.Identifier, node.Filesystem, node.MountLocation, node.Disk?.Manufacturer, node.Disk?.Model, node.Disk?.ConnectionType }
            .Any(s => s?.Contains(query, StringComparison.CurrentCultureIgnoreCase) == true);
    }
    private void NotifyBrowser()
    {
        foreach (var property in new[] { nameof(DeviceGroups), nameof(VisibleUnattachedVolumes), nameof(HasNoMatches), nameof(HasUnattachedVolumes), nameof(BrowserSummary), nameof(BrowserSelection) }) OnPropertyChanged(property);
    }
    private static string PropertySection(string label) => label switch
    {
        "Kind" or "Manufacturer" or "Model" or "Identifier" or "Health" or "Status" => "General",
        "Capacity" or "Used" or "Available" or "File system" or "Volume label" or "Partition scheme" or "Partitions" or "Disk number" or "Partition number" or "Partition type" => "Storage",
        "Connection" or "Internal" or "Removable" or "Online" or "Drive letter / mount" => "Connection",
        "System" or "Boot" or "Recovery" or "Hidden" or "Read only" or "Offline" or "Paging file" or "Supported operations" or "Action target" => "Protection & actions",
        _ => "Advanced"
    };
}

public sealed class DeviceGroup(string name, IEnumerable<StorageNode> nodes) : List<StorageNode>(nodes)
{
    public string Name { get; } = name;
}
public sealed record InspectorSection(string Title, IReadOnlyList<InspectorProperty> Items);
