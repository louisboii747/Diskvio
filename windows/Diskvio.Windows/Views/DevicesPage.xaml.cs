using System.ComponentModel;
using Diskvio.Windows.Models;
using Diskvio.Windows.Services;
using Diskvio.Windows.ViewModels;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;

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
            SidebarColumn.Width = compact ? new GridLength(1, GridUnitType.Star) : new GridLength(280);
            NavigationRow.Height = compact ? new GridLength(180) : new GridLength(1, GridUnitType.Star);
            DetailRow.Height = compact ? new GridLength(1, GridUnitType.Star) : new GridLength(0);
            Grid.SetColumnSpan(NavigationScroll, compact ? 2 : 1);
            Grid.SetColumn(DetailScroll, compact ? 0 : 1);
            Grid.SetRow(DetailScroll, compact ? 1 : 0);
            Grid.SetColumnSpan(DetailScroll, compact ? 2 : 1);
        };
    }
    private async void OnLoaded(object sender, RoutedEventArgs args) { await ViewModel.RefreshAsync(); _monitor.Start(); }
    private void OnUnloaded(object sender, RoutedEventArgs args) => _monitor.Stop();
    private void OnStateChanged(object? sender, PropertyChangedEventArgs args)
    {
        if (args.PropertyName is nameof(ViewModel.Layout) or nameof(ViewModel.SelectedNode))
        {
            Map.Update(ViewModel.Layout, ViewModel.SelectedNode?.Partition?.Device.Identifier);
            PartitionLegend.ItemsSource = ViewModel.SelectedDisk?.Children;
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
    private async void OnPartitionLegend(object sender, RoutedEventArgs args)
    { if (((Button)sender).Tag is StorageNode node) { ViewModel.SelectedNode = node; await ViewModel.QueryCapabilitiesAsync(); } }
    private async void OnDiskDetails(object sender, RoutedEventArgs args)
    { ViewModel.SelectDiskDetails(); await ViewModel.QueryCapabilitiesAsync(); }
    private async void OnMount(object sender, RoutedEventArgs args) => await ViewModel.ExecuteAsync(DiskAction.Mount, ConfirmAsync);
    private async void OnUnmount(object sender, RoutedEventArgs args) => await ViewModel.ExecuteAsync(DiskAction.Unmount, ConfirmAsync);
    private async Task<bool> ConfirmAsync(StorageNode node, DiskAction action)
    {
        var dialog = new ContentDialog
        {
            XamlRoot = XamlRoot, Title = $"{action} {node.Name}?",
            Content = action == DiskAction.Unmount
                ? $"Close files and applications using {node.MountLocation}. Unmounting removes its drive letter. The device identity will be checked again before proceeding."
                : "Windows will assign an available drive letter. The device identity will be checked again before proceeding.",
            PrimaryButtonText = action.ToString(), CloseButtonText = "Cancel", DefaultButton = ContentDialogButton.Close
        };
        return await dialog.ShowAsync() == ContentDialogResult.Primary;
    }
}
