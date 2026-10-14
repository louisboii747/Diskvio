using System.ComponentModel;
using Diskvio.Windows.Models;
using Diskvio.Windows.Services;
using Diskvio.Windows.ViewModels;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Controls.Primitives;
using Microsoft.UI.Xaml.Input;
using global::Windows.ApplicationModel.DataTransfer;

namespace Diskvio.Windows.Views;

public sealed partial class DevicesPage : Page
{
    private bool _synchronizingSelection;
    private bool _wasPlanningEditor;
    private double _sidebarWidth = 264;
    private readonly DispatcherTimer _monitor = new() { Interval = TimeSpan.FromSeconds(15) };
    public StorageWorkspaceViewModel ViewModel { get; } = new(new DiskService());
    public DevicesPage()
    {
        InitializeComponent(); DataContext = ViewModel;
        GroupedDevices.Source = ViewModel.DeviceGroups;
        ViewModel.PropertyChanged += OnStateChanged;
        Map.PartitionSelected += async partition => { ViewModel.SelectPartition(partition); await ViewModel.QueryCapabilitiesAsync(); };
        Map.RegionSelected += ViewModel.SelectRegion;
        Map.TargetContextRequested += async (segment, _) =>
        {
            if (ViewModel.IsBusy) return;
            if (segment.Partition is { } partition) ViewModel.SelectPartition(partition);
            else if (segment.Region is { } region) ViewModel.SelectRegion(region);
            var selected = ViewModel.SelectedNode;
            await ViewModel.QueryCapabilitiesAsync();
            if (selected is not null && ViewModel.SelectedNode == selected && !ViewModel.IsBusy)
                CreateNodeMenu(selected).ShowAt(Map);
        };
        _monitor.Tick += async (_, _) => await ViewModel.RefreshAsync();
        SizeChanged += (_, args) =>
        {
            UpdateLayout(args.NewSize.Width);
        };
    }
    private void UpdateLayout(double width)
    {
        var compact = width < 900;
        SidebarColumn.Width = compact ? new GridLength(1, GridUnitType.Star) : new GridLength(_sidebarWidth);
        SplitterColumn.Width = new GridLength(compact ? 0 : 8);
        SidebarSplitter.Visibility = compact ? Visibility.Collapsed : Visibility.Visible;
        NavigationRow.Height = compact ? new GridLength(200) : new GridLength(1, GridUnitType.Star);
        DetailRow.Height = compact ? new GridLength(1, GridUnitType.Star) : new GridLength(0);
        Grid.SetColumnSpan(NavigationScroll, compact ? 3 : 1);
        Grid.SetColumn(DetailScroll, compact ? 0 : 2);
        Grid.SetRow(DetailScroll, compact ? 1 : 0);
        Grid.SetColumnSpan(DetailScroll, compact ? 3 : 1);
        var wide = width - _sidebarWidth >= 920;
        var visible = InspectorToggle.IsChecked == true;
        InspectorColumn.Width = new GridLength(wide && visible ? 280 : 0);
        InspectorPanel.Visibility = visible ? Visibility.Visible : Visibility.Collapsed;
        InspectorPanel.Padding = wide ? new Thickness(20, 0, 0, 0) : new Thickness(0, 20, 0, 0);
        InspectorPanel.BorderThickness = wide ? new Thickness(1, 0, 0, 0) : new Thickness(0, 1, 0, 0);
        Grid.SetColumn(InspectorPanel, wide ? 1 : 0);
        Grid.SetRow(InspectorPanel, wide ? 0 : 1);
    }
    private void OnToggleInspector(object sender, RoutedEventArgs args) => UpdateLayout(ActualWidth);
    private void OnResultClosed(InfoBar sender, InfoBarClosedEventArgs args) => ViewModel.DismissResultMessage();
    private void OnDeviceNoticeClosed(InfoBar sender, InfoBarClosedEventArgs args) => ViewModel.DismissDeviceNotice();
    private void OnSidebarDrag(object sender, DragDeltaEventArgs args) => ResizeSidebar(args.HorizontalChange);
    private void ResizeSidebar(double change)
    {
        _sidebarWidth = Math.Clamp(_sidebarWidth + change, 220, Math.Min(380, Math.Max(220, ActualWidth - 540)));
        UpdateLayout(ActualWidth);
    }
    private void OnSidebarSplitterKeyDown(object sender, KeyRoutedEventArgs args)
    {
        if (args.Key is not (global::Windows.System.VirtualKey.Left or global::Windows.System.VirtualKey.Right)) return;
        ResizeSidebar(args.Key == global::Windows.System.VirtualKey.Left ? -16 : 16);
        args.Handled = true;
    }
    private void OnFilterChanged(object sender, SelectionChangedEventArgs args)
    {
        if (sender is ComboBox { SelectedItem: ComboBoxItem { Content: string filter } }) ViewModel.DeviceFilter = filter;
    }
    private void OnSortChanged(object sender, SelectionChangedEventArgs args)
    {
        if (sender is ComboBox { SelectedItem: ComboBoxItem { Content: string sort } }) ViewModel.PartitionSort = sort;
    }
    private void OnCopyIdentifier(object sender, RoutedEventArgs args) => Copy(ViewModel.SelectedIdentifier);
    private void OnCopyProperty(object sender, RoutedEventArgs args)
    {
        if (sender is FrameworkElement { DataContext: InspectorProperty property }) Copy(property.Value);
    }
    private static void Copy(string value)
    {
        if (string.IsNullOrEmpty(value)) return;
        var data = new DataPackage(); data.SetText(value); Clipboard.SetContent(data);
    }
    private async void OnLoaded(object sender, RoutedEventArgs args) { await ViewModel.RefreshAsync(); _monitor.Start(); }
    private void OnUnloaded(object sender, RoutedEventArgs args) { _monitor.Stop(); ViewModel.CancelPartitionPreview(); ViewModel.RevokeTestDiskDesignation(); }
    private void OnStateChanged(object? sender, PropertyChangedEventArgs args)
    {
        if (args.PropertyName == nameof(ViewModel.DeviceGroups)) GroupedDevices.Source = ViewModel.DeviceGroups;
        if (args.PropertyName == nameof(ViewModel.IsPlanningEditor))
        {
            if (ViewModel.IsPlanningEditor && !_wasPlanningEditor) { PlanningExpander.IsExpanded = true; PlanningWorkspace.StartBringIntoView(); }
            _wasPlanningEditor = ViewModel.IsPlanningEditor;
        }
        if (args.PropertyName is nameof(ViewModel.Layout) or nameof(ViewModel.SelectedNode) or nameof(ViewModel.Children) or nameof(ViewModel.DeviceGroups))
        {
            Map.Update(ViewModel.Layout, ViewModel.SelectedMapId);
            _synchronizingSelection = true;
            try
            {
                ChildList.SelectedItem = (ChildList.ItemsSource as IEnumerable<StorageNode>)?.FirstOrDefault(n => n.Device.Identifier == ViewModel.SelectedNode?.Device.Identifier);
                DeviceList.SelectedItem = DeviceList.Items.OfType<StorageNode>().FirstOrDefault(n => n.Device.Identifier == ViewModel.BrowserSelection?.Device.Identifier);
            }
            finally { _synchronizingSelection = false; }
        }
    }
    private async void OnDiskSelectionChanged(object sender, SelectionChangedEventArgs args)
    {
        if (_synchronizingSelection || args.AddedItems.FirstOrDefault() is not StorageNode node) return;
        ViewModel.SelectBrowserNode(node);
        await ViewModel.QueryCapabilitiesAsync();
    }
    private async void OnExplorerSelectionChanged(object sender, SelectionChangedEventArgs args)
    {
        if (_synchronizingSelection || args.AddedItems.FirstOrDefault() is not StorageNode node) return;
        ViewModel.SelectedNode = node;
        await ViewModel.QueryCapabilitiesAsync();
    }
    private async void OnUnattachedSelected(object sender, SelectionChangedEventArgs args)
    { if (UnattachedList.SelectedItem is StorageNode node) { ViewModel.SelectedDisk = null; ViewModel.SelectedNode = node; await ViewModel.QueryCapabilitiesAsync(); } }
    private async void OnDiskDetails(object sender, RoutedEventArgs args)
    {
        ViewModel.SelectDiskDetails(); InspectorToggle.IsChecked = true; UpdateLayout(ActualWidth);
        await ViewModel.QueryCapabilitiesAsync();
    }
    private async void OnMount(object sender, RoutedEventArgs args) => await ViewModel.ExecuteAsync(DiskAction.Mount, ConfirmAsync);
    private async void OnUnmount(object sender, RoutedEventArgs args) => await ViewModel.ExecuteAsync(DiskAction.Unmount, ConfirmAsync);
    private async void OnRename(object sender, RoutedEventArgs args) => await ViewModel.ExecuteWithParametersAsync(DiskAction.RenameVolume, ConfirmChangeAsync);
    private async void OnDriveLetter(object sender, RoutedEventArgs args) => await ViewModel.ExecuteWithParametersAsync(DiskAction.SetDriveLetter, ConfirmChangeAsync);

    private async void OnContextRequested(UIElement sender, ContextRequestedEventArgs args)
    {
        if (ViewModel.IsBusy || sender is not FrameworkElement element) return;
        var node = element.DataContext as StorageNode ?? (element as ListView)?.SelectedItem as StorageNode;
        if (node is null) return;
        args.Handled = true;
        var positioned = args.TryGetPosition(this, out var position);
        ViewModel.SelectBrowserNode(node);
        await ViewModel.QueryCapabilitiesAsync();
        if (ViewModel.IsBusy || ViewModel.SelectedNode != node) return;
        CreateNodeMenu(node).ShowAt(this, positioned ? new FlyoutShowOptions { Position = position } : new FlyoutShowOptions());
    }

    private MenuFlyout CreateNodeMenu(StorageNode node)
    {
        var menu = new MenuFlyout();
        AddPlanningItems(menu);
        void Add(string text, bool enabled, RoutedEventHandler handler)
        {
            var item = new MenuFlyoutItem { Text = text, IsEnabled = enabled };
            item.Click += handler; menu.Items.Add(item);
            if (!enabled) ToolTipService.SetToolTip(item, ViewModel.ActionExplanation);
        }
        if (node.Kind is "Volume" or "Partition")
        {
            Add("Rename volume label…", ViewModel.CanRename, OnRename);
            Add("Assign or change drive letter…", ViewModel.CanSetDriveLetter, OnDriveLetter);
            Add("Mount volume…", ViewModel.CanMount, OnMount);
            Add("Unmount volume…", ViewModel.CanUnmount, OnUnmount);
            menu.Items.Add(new MenuFlyoutSeparator());
        }
        Add("Copy technical identifier", true, (_, _) => { var data = new DataPackage(); data.SetText(node.Device.Identifier); Clipboard.SetContent(data); });
        Add("Refresh disks", !ViewModel.IsBusy, async (_, _) => await ViewModel.RefreshAsync());
        return menu;
    }

    private void AddPlanningItems(MenuFlyout menu)
    {
        var inspect = new MenuFlyoutItem { Text = "Inspect partition layout", Command = ViewModel.InspectPartitionPlanningCommand };
        menu.Items.Add(inspect);
        if (ViewModel.SelectedNode?.Region is not null)
        {
            var create = new MenuFlyoutItem { Text = "Create partition preview", IsEnabled = ViewModel.CanBeginCreatePreview };
            create.Click += (_, _) => ViewModel.BeginCreatePreview();
            ToolTipService.SetToolTip(create, ViewModel.RegionSummary);
            menu.Items.Add(create);
        }
        if (ViewModel.SelectedNode?.Partition is not null)
        {
            var delete = new MenuFlyoutItem { Text = "Inspect deletion preview", IsEnabled = ViewModel.CanBeginDeletePreview };
            delete.Click += (_, _) => ViewModel.BeginDeletePreview();
            ToolTipService.SetToolTip(delete, "Preview only. Protected targets remain ineligible.");
            menu.Items.Add(delete);
        }
        menu.Items.Add(new MenuFlyoutSeparator());
    }

    private async Task<OperationInput?> ConfirmChangeAsync(StorageNode node, DiskAction action)
    {
        var rename = action == DiskAction.RenameVolume;
        var panel = new StackPanel { Spacing = 12, MinWidth = 300 };
        panel.Children.Add(new TextBlock { Text = $"{node.Name} · Disk {node.Disk?.Number} · Partition {node.Partition?.Number}\n{node.Device.Identifier}", TextWrapping = TextWrapping.Wrap, IsTextSelectionEnabled = true });
        var label = new TextBox { Header = "New volume label", Text = node.Volume?.Label ?? "", MaxLength = ViewModel.LabelMaxLength };
        var letter = new ComboBox { Header = "New drive letter", HorizontalAlignment = HorizontalAlignment.Stretch };
        foreach (var value in Enumerable.Range('D', 'Z' - 'D' + 1).Select(c => ((char)c).ToString()).Where(l => l != node.Device.DriveLetter)) letter.Items.Add(value);
        panel.Children.Add(rename ? label : letter);
        var preview = new TextBlock { TextWrapping = TextWrapping.Wrap };
        panel.Children.Add(preview);
        panel.Children.Add(new TextBlock { Text = rename
            ? $"Maximum {ViewModel.LabelMaxLength} characters. An empty label removes the current label. The backend will validate this change again before applying it."
            : "Close all files on this volume. Changing its letter can break shortcuts and application paths. Occupied letters are rejected. The backend will lock and validate the volume before applying the change.", TextWrapping = TextWrapping.Wrap });
        var dialog = new ContentDialog { XamlRoot = XamlRoot, Title = rename ? "Rename volume label" : "Change drive letter", Content = panel,
            PrimaryButtonText = rename ? "Apply label" : "Apply drive letter", CloseButtonText = "Cancel", DefaultButton = ContentDialogButton.Close };
        void UpdatePreview()
        {
            var newValue = rename ? label.Text : letter.SelectedItem as string;
            dialog.IsPrimaryButtonEnabled = rename ? label.Text != (node.Volume?.Label ?? "") && label.Text.Trim() == label.Text && !label.Text.Any(c => char.IsControl(c) || "\\/:*?\"<>|+.,;=[]".Contains(c)) : newValue is not null;
            preview.Text = rename ? $"Preview: {Display(node.Volume?.Label)} → {Display(label.Text)}" : $"Preview: {node.Location} → {newValue ?? "Choose a letter"}{(newValue is null ? "" : ":")}";
        }
        label.TextChanged += (_, _) => UpdatePreview(); letter.SelectionChanged += (_, _) => UpdatePreview(); UpdatePreview();
        if (await dialog.ShowAsync() != ContentDialogResult.Primary) return null;
        return rename ? new(VolumeLabel: label.Text) : new(DriveLetter: (string)letter.SelectedItem);
    }
    private static string Display(string? label) => string.IsNullOrEmpty(label) ? "(no label)" : label;
    private async Task<bool> ConfirmAsync(StorageNode node, DiskAction action)
    {
        var dialog = new ContentDialog
        {
            XamlRoot = XamlRoot, Title = $"{action} {node.Name}?",
            Content = $"{node.Name} · Disk {node.Disk?.Number} · Partition {node.Partition?.Number} · {node.Capacity}\n{node.Device.Identifier}\n\n" + (action == DiskAction.Unmount
                ? $"Close files and applications using {node.MountLocation}. Unmounting removes its drive letter. The device identity will be checked again before proceeding."
                : "Windows will assign an available drive letter. The device identity will be checked again before proceeding."),
            PrimaryButtonText = action.ToString(), CloseButtonText = "Cancel", DefaultButton = ContentDialogButton.Close
        };
        return await dialog.ShowAsync() == ContentDialogResult.Primary;
    }
}
