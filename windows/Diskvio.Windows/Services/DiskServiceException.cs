using Diskvio.Windows.Models;

namespace Diskvio.Windows.Services;

public sealed class DiskServiceException(string message, Exception? innerException = null, BackendOperationError? backendError = null)
    : Exception(message, innerException)
{
    public BackendOperationError? BackendError { get; } = backendError;
}
