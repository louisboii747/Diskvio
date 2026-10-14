using Diskvio.Windows.Models;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Automation;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Media;

namespace Diskvio.Windows.Controls;

public sealed class PartitionMap : UserControl
{
    public event Action<StoragePartition>? PartitionSelected;
    public event Action<AllocatableRegion>? RegionSelected;
    public event Action<PartitionSegment, FrameworkElement>? TargetContextRequested;
    public bool IsPreview { get; set; }
    private readonly Canvas _canvas = new() { Height = 102 };
    private PartitionLayout? _layout;
    private string? _selectedId;
    public PartitionMap()
    {
        Content = new ScrollViewer { Content = _canvas, HorizontalScrollBarVisibility = ScrollBarVisibility.Auto, VerticalScrollBarVisibility = ScrollBarVisibility.Disabled };
        Height = 122;
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
            var tone = segment.Partition is { } colored ? PartitionPresentation.Tone(colored) : "Unknown";
            var categoryBrush = (Brush)Application.Current.Resources[tone != "Unknown" ? $"Partition{tone}Brush" : BrushKey(segment.Category)];
            FrameworkElement element;
            if (segment.Partition is not null || segment.Region is not null || segment.Proposal is not null)
            {
                var content = new Grid { RowSpacing = 8, HorizontalAlignment = HorizontalAlignment.Stretch };
                content.RowDefinitions.Add(new() { Height = new GridLength(5) });
                content.RowDefinitions.Add(new() { Height = new GridLength(1, GridUnitType.Star) });
                content.Children.Add(new Border { Background = categoryBrush, CornerRadius = new CornerRadius(2) });
                if (width >= 90)
                {
                    var labels = new StackPanel { Spacing = 3, Margin = new Thickness(6, 0, 22, 0) };
                    labels.Children.Add(new TextBlock { Text = segment.Label, TextTrimming = TextTrimming.CharacterEllipsis, FontWeight = Microsoft.UI.Text.FontWeights.SemiBold });
                    labels.Children.Add(new TextBlock { Text = $"{segment.DisplayCapacity} · {segment.KindLabel}", FontSize = 12, TextTrimming = TextTrimming.CharacterEllipsis });
                    if (segment.Partition is { } existing)
                    {
                        var mounts = string.Join(", ", existing.Volumes.Select(v => v.Device.DriveLetter is { } letter ? $"{letter}:" : v.Device.MountPoint).Where(m => !string.IsNullOrWhiteSpace(m)));
                        labels.Children.Add(new TextBlock { Text = $"{PartitionPresentation.Filesystems(existing)} {mounts}".Trim(), FontSize = 12, TextTrimming = TextTrimming.CharacterEllipsis });
                    }
                    Grid.SetRow(labels, 1); content.Children.Add(labels);
                }
                else
                {
                    var label = new TextBlock { Text = segment.Partition?.Number?.ToString(System.Globalization.CultureInfo.CurrentCulture) ?? (segment.Partition is not null ? "?" : segment.Region is not null ? "Free" : "Plan"), HorizontalAlignment = HorizontalAlignment.Center, VerticalAlignment = VerticalAlignment.Center };
                    Grid.SetRow(label, 1); content.Children.Add(label);
                }
                var button = new Button { Content = content, Tag = segment.SelectionId, Width = Math.Max(1, width - 3), Height = 102,
                    Padding = new Thickness(4), MinWidth = 0, HorizontalContentAlignment = HorizontalAlignment.Stretch,
                    VerticalContentAlignment = VerticalAlignment.Stretch,
                    BorderThickness = new Thickness(!IsPreview && _selectedId == segment.SelectionId ? 2 : 1),
                    BorderBrush = (Brush)Application.Current.Resources[!IsPreview && _selectedId == segment.SelectionId ? "SystemControlHighlightAccentBrush" : "ControlStrokeColorDefaultBrush"] };
                AutomationProperties.SetItemStatus(button, !IsPreview && _selectedId == segment.SelectionId ? "Selected" : "Not selected");
                if (!IsPreview && _selectedId == segment.SelectionId && width >= 90)
                {
                    var selected = new FontIcon { Glyph = "\uE73E", FontSize = 12, HorizontalAlignment = HorizontalAlignment.Right,
                        VerticalAlignment = VerticalAlignment.Bottom, Margin = new Thickness(0, 0, 4, 4), Foreground = button.BorderBrush };
                    Grid.SetRow(selected, 1);
                    content.Children.Add(selected);
                }
                AutomationProperties.SetHelpText(button, IsPreview ? "Read-only proposed layout. Nothing has been applied." : "Select to inspect this partition or region. Use arrow keys to move focus and Enter or Space to select. Small targets have a minimum visual width.");
                button.Click += (_, _) =>
                {
                    if (IsPreview) return;
                    if (segment.Partition is { } partition) PartitionSelected?.Invoke(partition);
                    else if (segment.Region is { } region) RegionSelected?.Invoke(region);
                };
                button.IsTabStop = !IsPreview;
                button.ContextRequested += (_, args) =>
                {
                    if (IsPreview) return;
                    args.Handled = true;
                    TargetContextRequested?.Invoke(segment, button);
                };
                button.KeyDown += (_, args) =>
                {
                    if (args.Key is not (global::Windows.System.VirtualKey.Left or global::Windows.System.VirtualKey.Right or global::Windows.System.VirtualKey.Home or global::Windows.System.VirtualKey.End)) return;
                    var buttons = _canvas.Children.OfType<Button>().ToList();
                    var index = buttons.IndexOf(button);
                    var next = args.Key switch
                    {
                        global::Windows.System.VirtualKey.Home => 0,
                        global::Windows.System.VirtualKey.End => buttons.Count - 1,
                        global::Windows.System.VirtualKey.Left => Math.Max(0, index - 1),
                        _ => Math.Min(buttons.Count - 1, index + 1)
                    };
                    buttons[next].Focus(FocusState.Keyboard);
                    args.Handled = true;
                };
                element = button;
            }
            else element = new Border { Width = Math.Max(1, width - 1), Height = 102, Background = categoryBrush, CornerRadius = new CornerRadius(3) };
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
        "proposal" => "SystemControlHighlightAccentBrush", "unallocated" => "PartitionReservedBrush",
        "unmapped" => "PartitionUnmappedBrush", _ => "PartitionUnknownBrush"
    };
}
