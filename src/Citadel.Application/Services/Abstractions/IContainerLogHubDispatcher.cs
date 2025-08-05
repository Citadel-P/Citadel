namespace Application.Services.Abstractions;

public interface IContainerLogHubDispatcher
{
    Task SendContainerLog(string containerId, string logLine);
    
    Task SendContainerLogsBatch(string containerId, IEnumerable<string> recentLogs);
}