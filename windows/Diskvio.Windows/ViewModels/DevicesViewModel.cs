using System.Collections.ObjectModel;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using Diskvio.Windows.Models;
using Diskvio.Windows.Services;

namespace Diskvio.Windows.ViewModels;

public sealed class DevicesViewModel : ObservableObject
{
    private readonly IDiskService _service;
    private Disk? _selectedDisk;
    private bool _isLoading;
    private bool _hasLoaded;
    private string _errorMessage = "";
    private string _lastUpdated = "";

    public DevicesViewModel(IDiskService service)
    {
        _service = service;
        RefreshCommand = new AsyncRelayCommand(RefreshAsync);
    }

    public ObservableCollection<Disk> Disks { get; } = [];
    public IAsyncRelayCommand RefreshCommand { get; }
    public Disk? SelectedDisk
    {
        get => _selectedDisk;
        set
        {
            if (SetProperty(ref _selectedDisk, value)) OnPropertyChanged(nameof(HasSelection));
        }
    }
    public bool IsLoading => _isLoading;
    public bool CanInspect => !IsLoading;
    public bool HasSelection => SelectedDisk is not null;
    public bool HasError => !string.IsNullOrEmpty(ErrorMessage);
    public bool IsEmpty => _hasLoaded && !IsLoading && !HasError && Disks.Count == 0;
    public bool IsWorkspaceVisible => Disks.Count > 0;
    public string ErrorMessage => _errorMessage;
    public string DeviceCount => $"{Disks.Count} {(Disks.Count == 1 ? "device" : "devices")}";
    public string LastUpdated => _lastUpdated;

    private async Task RefreshAsync()
    {
        if (_isLoading) return;
        var selectedNumber = SelectedDisk?.Number;
        _isLoading = true;
        _errorMessage = "";
        NotifyState();
        try
        {
            var disks = await _service.ListDisksAsync();
            Disks.Clear();
            foreach (var disk in disks.OrderBy(disk => disk.Number)) Disks.Add(disk);
            SelectedDisk = Disks.FirstOrDefault(disk => disk.Number == selectedNumber) ?? Disks.FirstOrDefault();
            _lastUpdated = $"Updated {DateTime.Now:t}";
        }
        catch (Exception error)
        {
            // Failed refreshes must not present stale disks as currently attached.
            Disks.Clear();
            SelectedDisk = null;
            _lastUpdated = "";
            _errorMessage = error.Message;
        }
        finally
        {
            _hasLoaded = true;
            _isLoading = false;
            NotifyState();
        }
    }

    private void NotifyState()
    {
        OnPropertyChanged(nameof(IsLoading));
        OnPropertyChanged(nameof(CanInspect));
        OnPropertyChanged(nameof(HasError));
        OnPropertyChanged(nameof(ErrorMessage));
        OnPropertyChanged(nameof(IsEmpty));
        OnPropertyChanged(nameof(IsWorkspaceVisible));
        OnPropertyChanged(nameof(DeviceCount));
        OnPropertyChanged(nameof(LastUpdated));
    }
}
