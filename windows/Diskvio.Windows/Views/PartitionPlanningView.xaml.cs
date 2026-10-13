using System.ComponentModel;
using Diskvio.Windows.ViewModels;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;

namespace Diskvio.Windows.Views;

public sealed partial class PartitionPlanningView : UserControl
{
    private StorageWorkspaceViewModel? _workspace;

    public PartitionPlanningView()
    {
        InitializeComponent();
        BeforeMap.IsPreview = true;
        AfterMap.IsPreview = true;
        Loaded += (_, _) => Connect();
        DataContextChanged += (_, _) => Connect();
        Unloaded += (_, _) =>
        {
            if (_workspace is not null) _workspace.PropertyChanged -= OnStateChanged;
            _workspace = null;
        };
    }

    private void Connect()
    {
        if (_workspace is not null) _workspace.PropertyChanged -= OnStateChanged;
        _workspace = DataContext as StorageWorkspaceViewModel;
        if (_workspace is not null) _workspace.PropertyChanged += OnStateChanged;
        UpdateMaps();
    }

    private void OnStateChanged(object? sender, PropertyChangedEventArgs args)
    {
        if (args.PropertyName is nameof(StorageWorkspaceViewModel.PreviewLayout) or nameof(StorageWorkspaceViewModel.Layout)) UpdateMaps();
    }

    private void UpdateMaps()
    {
        BeforeMap.Update(_workspace?.Layout, null);
        AfterMap.Update(_workspace?.PreviewLayout, null);
    }

    private async void OnDesignate(object sender, RoutedEventArgs args)
    {
        if (_workspace is not { CanDesignateTestDisk: true, PlanningDiscovery.PhysicalIdentity: { } identity } workspace) return;
        var discovery = workspace.PlanningDiscovery;
        var dialog = new ContentDialog
        {
            XamlRoot = XamlRoot, Title = "Designate expendable external test media?",
            Content = workspace.PlanningDiskIdentity + "\n\nConfirm that this exact disk is expendable test media. The designation lasts only for this planning session and clears on refresh or disk change. It cannot override backend protection. This milestone changes no storage.",
            PrimaryButtonText = "Designate for previews", CloseButtonText = "Cancel", DefaultButton = ContentDialogButton.Close
        };
        if (await dialog.ShowAsync() == ContentDialogResult.Primary && ReferenceEquals(workspace.PlanningDiscovery, discovery)) workspace.DesignateTestDisk(identity);
    }

    private void OnRevoke(object sender, RoutedEventArgs args) => _workspace?.RevokeTestDiskDesignation();
    private void OnCreate(object sender, RoutedEventArgs args) => _workspace?.BeginCreatePreview();
    private void OnDelete(object sender, RoutedEventArgs args) => _workspace?.BeginDeletePreview();
    private async void OnPreview(object sender, RoutedEventArgs args) { if (_workspace is { } workspace) await workspace.PreviewPartitionAsync(); }
    private void OnReset(object sender, RoutedEventArgs args) => _workspace?.ResetPartitionPreview();
    private void OnCancel(object sender, RoutedEventArgs args) => _workspace?.CancelPartitionPreview();
}
