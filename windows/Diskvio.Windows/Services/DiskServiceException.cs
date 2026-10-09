namespace Diskvio.Windows.Services;

public sealed class DiskServiceException(string message, Exception? innerException = null)
    : Exception(message, innerException);
