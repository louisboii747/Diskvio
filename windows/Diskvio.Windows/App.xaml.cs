using Diskvio.Windows.Services;
using Microsoft.UI.Xaml;

namespace Diskvio.Windows;

public partial class App : Application
{
    private Window? _window;
    public App()
    {
        UnhandledException += (_, args) => AppDiagnostics.Record(args.Exception, args.Message);
        try { InitializeComponent(); }
        catch (Exception error) { AppDiagnostics.Record(error, "Application resources failed"); throw; }
    }
    protected override void OnLaunched(LaunchActivatedEventArgs args)
    {
        try { _window = new MainWindow(); _window.Activate(); }
        catch (Exception error) { AppDiagnostics.Record(error, "Startup failed"); throw; }
    }
}
