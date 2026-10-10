using Diskvio.Windows.Models;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Automation;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Media;

namespace Diskvio.Windows.Controls;

// Native buttons retain focus, keyboard activation, high-contrast and theme states.
public sealed class PartitionMap : UserControl
{
    public event Action<StoragePartition>? PartitionSelected;
    private readonly Canvas _canvas = new();
    private PartitionLayout? _layout;
    private string? _selectedId;
    public PartitionMap() { Content = _canvas; Height = 76; SizeChanged += (_, _) => Render(); }
    public void Update(PartitionLayout? layout, string? selectedId)
    { _layout = layout; _selectedId = selectedId; Render(); }
    private void Render()
    {
        _canvas.Children.Clear();
        if (_layout is null || ActualWidth <= 0) return;
        var total = _layout.Segments.Sum(s => (double)s.Length);
        if (total <= 0) return;
        double left = 0;
        foreach (var segment in _layout.Segments)
        {
            var width = ActualWidth * segment.Length / total;
            FrameworkElement element;
            if (segment.Partition is { } partition)
            {
                var button = new Button { Content = segment.Label, Width = Math.Max(0, width), Height = 76,
                    Padding = new Thickness(0), MinWidth = 0, HorizontalContentAlignment = HorizontalAlignment.Center };
                if (_selectedId == partition.Device.Identifier)
                    button.Style = (Style)Application.Current.Resources["AccentButtonStyle"];
                button.Click += (_, _) => PartitionSelected?.Invoke(partition);
                element = button;
            }
            else element = new Border { Width = width, Height = 76, Background = (Brush)Application.Current.Resources["ControlFillColorSecondaryBrush"] };
            ToolTipService.SetToolTip(element, segment.Description);
            AutomationProperties.SetName(element, segment.Description);
            Canvas.SetLeft(element, left);
            _canvas.Children.Add(element);
            left += width;
        }
    }
}
