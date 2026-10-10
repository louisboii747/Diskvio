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
    private readonly DispatcherTimer _monitor = new() { Interval = TimeSpan.FromSeconds(15) };
    public StorageWorkspaceViewModel ViewModel { get; } = new(new DiskService());
    public DevicesPage()
    {
        InitializeComponent(); DataContext = ViewModel;
        ViewModel.PropertyChanged += OnStateChanged;
        Map.PartitionSelected += async partition => { ViewModel.SelectPartition(partition); await ViewModel.QueryCapabilitiesAsync(); };
        _monitor.Tick += async (_, _) => { if (!ViewModel.IsBusy) await ViewModel.RefreshAsync(); };
        SizeChanged += (_, args) =>
        {
            var compact = args.NewSize.Width < 850;
            SidebarColumn.Width = compact ? new GridLength(1, GridUnitType.Star) : new GridLength(240);
            NavigationRow.Height = compact ? new GridLength(180) : new GridLength(1, GridUnitType.Star);
            DetailRow.Height = compact ? new GridLength(1, GridUnitType.Star) : new GridLength(0);
            Grid.SetColumnSpan(NavigationScroll, compact ? 2 : 1);
            Grid.SetColumn(DetailScroll, compact ? 0 : 1);
            Grid.SetRow(DetailScroll, compact ? 1 : 0);
            Grid.SetColumnSpan(DetailScroll, compact ? 2 : 1);
            var wide = args.NewSize.Width >= 1180;
            InspectorColumn.Width = new GridLength(wide ? 280 : 0);
            Grid.SetColumn(InspectorPanel, wide ? 1 : 0);
            Grid.SetRow(InspectorPanel, wide ? 0 : 1);
        };
    }
    private async void OnLoaded(object sender, RoutedEventArgs args) { await ViewModel.RefreshAsync(); _monitor.Start(); }
    private void OnUnloaded(object sender, RoutedEventArgs args) => _monitor.Stop();
    private void OnStateChanged(object? sender, PropertyChangedEventArgs args)
    {
        if (args.PropertyName is nameof(ViewModel.Layout) or nameof(ViewModel.SelectedNode))
        {
            Map.Update(ViewModel.Layout, ViewModel.SelectedNode?.Partition?.Device.Identifier);
            _synchronizingSelection = true;
            try { ChildList.SelectedItem = ViewModel.Children.FirstOrDefault(n => n.Device.Identifier == ViewModel.SelectedNode?.Device.Identifier); }
            finally { _synchronizingSelection = false; }
        }
    }
    private async void OnDiskSelectionChanged(object sender, SelectionChangedEventArgs args)
    { await ViewModel.QueryCapabilitiesAsync(); }
    private async void OnExplorerSelectionChanged(object sender, SelectionChangedEventArgs args)
    {
        if (_synchronizingSelection || args.AddedItems.FirstOrDefault() is not StorageNode node) return;
        ViewModel.SelectedNode = node;
        await ViewModel.QueryCapabilitiesAsync();
    }
    private async void OnUnattachedSelected(object sender, SelectionChangedEventArgs args)
    { if (UnattachedList.SelectedItem is StorageNode node) { ViewModel.SelectedDisk = null; ViewModel.SelectedNode = node; await ViewModel.QueryCapabilitiesAsync(); } }
    private async void OnDiskDetails(object sender, RoutedEventArgs args)
    { ViewModel.SelectDiskDetails(); await ViewModel.QueryCapabilitiesAsync(); }
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
        var positioned = args.TryGetPosition(element, out var position);
        if (node.Kind == "Physical disk") ViewModel.SelectedDisk = node;
        else if (node.Disk is null) { ViewModel.SelectedDisk = null; ViewModel.SelectedNode = node; }
        else ViewModel.SelectedNode = node;
        await ViewModel.QueryCapabilitiesAsync();
        if (ViewModel.IsBusy || ViewModel.SelectedNode != node) return;
        var menu = new MenuFlyout();
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
        menu.ShowAt(element, positioned ? new FlyoutShowOptions { Position = position } : new FlyoutShowOptions());
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
