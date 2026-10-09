using Diskvio.Windows.Services;
using Diskvio.Windows.ViewModels;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;

namespace Diskvio.Windows.Views;

public sealed partial class DevicesPage : Page
{
    private bool _initialized;
    public DevicesViewModel ViewModel { get; } = new(new DiskService());

    public DevicesPage()
    {
        InitializeComponent();
        DataContext = ViewModel;
    }

    private async void OnLoaded(object sender, RoutedEventArgs args)
    {
        if (_initialized) return;
        _initialized = true;
        await ViewModel.RefreshCommand.ExecuteAsync(null);
    }

    private void OnDiskSelectionChanged(object sender, SelectionChangedEventArgs args)
    {
        // Wait for ListView's item layout after refresh before bringing the
        // selected row into view, especially in the shorter stacked layout.
        DispatcherQueue.TryEnqueue(() =>
        {
            if (ViewModel.SelectedDisk is { } disk) DeviceList.ScrollIntoView(disk);
        });
    }
}
