namespace Diskvio.Windows.Services;

internal static class AppDiagnostics
{
    public static void Record(Exception exception, string? message = null)
    {
        try
        {
            var directory = Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData), "Diskvio");
            Directory.CreateDirectory(directory);
            File.AppendAllText(Path.Combine(directory, "application-errors.log"), $"{DateTimeOffset.Now:O} {message}{Environment.NewLine}{exception}{Environment.NewLine}");
        }
        catch { /* Logging must never replace the original failure. */ }
    }
}
