using Diskvio.Windows.Models;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Automation;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Media;

namespace Diskvio.Windows.Controls;

public sealed class PartitionMap : UserControl
{
    public event Action<StoragePartition>? PartitionSelected;
    private readonly Canvas _canvas = new() { Height = 84 };
    private PartitionLayout? _layout;
    private string? _selectedId;
    public PartitionMap()
    {
        Content = new ScrollViewer { Content = _canvas, HorizontalScrollBarVisibility = ScrollBarVisibility.Auto, VerticalScrollBarVisibility = ScrollBarVisibility.Disabled };
        Height = 104;
        SizeChanged += (_, _) => Render();
        ActualThemeChanged += (_, _) => Render();
    }
    public void Update(PartitionLayout? layout, string? selectedId)
    { _layout = layout; _selectedId = selectedId; Render(); }

    private void Render()
    {
        var focused = _canvas.Children.OfType<Button>().FirstOrDefault(b => b.FocusState != FocusState.Unfocused)?.Tag as string;
        _canvas.Children.Clear();
        if (_layout is null || ActualWidth <= 0) { _canvas.Width = 0; return; }
        var widths = _layout.VisualWidths(ActualWidth);
        _canvas.Width = widths.Sum();
        double left = 0;
        for (var i = 0; i < _layout.Segments.Count; i++)
        {
            var segment = _layout.Segments[i];
            var width = widths[i];
            var categoryBrush = (Brush)Application.Current.Resources[BrushKey(segment.Category)];
            FrameworkElement element;
            if (segment.Partition is { } partition)
            {
                var content = new Grid { RowSpacing = 8, HorizontalAlignment = HorizontalAlignment.Stretch };
                content.RowDefinitions.Add(new() { Height = new GridLength(5) });
                content.RowDefinitions.Add(new() { Height = new GridLength(1, GridUnitType.Star) });
                content.Children.Add(new Border { Background = categoryBrush, CornerRadius = new CornerRadius(2) });
                if (width >= 90)
                {
                    var labels = new StackPanel { Spacing = 3, Margin = new Thickness(6, 0, 6, 0) };
                    labels.Children.Add(new TextBlock { Text = segment.Label, TextTrimming = TextTrimming.CharacterEllipsis, FontWeight = Microsoft.UI.Text.FontWeights.SemiBold });
                    labels.Children.Add(new TextBlock { Text = Disk.FormatCapacity(segment.Length), FontSize = 12, TextTrimming = TextTrimming.CharacterEllipsis });
                    Grid.SetRow(labels, 1); content.Children.Add(labels);
                }
                var button = new Button { Content = content, Tag = partition.Device.Identifier, Width = Math.Max(1, width - 2), Height = 84,
                    Padding = new Thickness(4), MinWidth = 0, HorizontalContentAlignment = HorizontalAlignment.Stretch,
                    VerticalContentAlignment = VerticalAlignment.Stretch,
                    BorderThickness = new Thickness(_selectedId == partition.Device.Identifier ? 2 : 1),
                    BorderBrush = (Brush)Application.Current.Resources[_selectedId == partition.Device.Identifier ? "SystemControlHighlightAccentBrush" : "ControlStrokeColorDefaultBrush"] };
                AutomationProperties.SetHelpText(button, "Select to inspect this partition. Small partitions have a minimum visual width.");
                button.Click += (_, _) => PartitionSelected?.Invoke(partition);
                element = button;
            }
            else element = new Border { Width = Math.Max(1, width - 1), Height = 84, Background = categoryBrush, CornerRadius = new CornerRadius(3) };
            ToolTipService.SetToolTip(element, new TextBlock { Text = segment.Description, MaxWidth = 420, TextWrapping = TextWrapping.Wrap });
            AutomationProperties.SetName(element, segment.Description);
            Canvas.SetLeft(element, left); _canvas.Children.Add(element); left += width;
            if (element is Button focusButton && Equals(focusButton.Tag, focused)) focusButton.Focus(FocusState.Programmatic);
        }
    }

    public static string BrushKey(string category) => category switch
    {
        "efi_system" => "PartitionSystemBrush", "recovery" => "PartitionRecoveryBrush",
        "microsoft_reserved" => "PartitionReservedBrush", "basic_data" => "PartitionDataBrush",
        "unmapped" => "PartitionUnmappedBrush", _ => "PartitionUnknownBrush"
    };
}
