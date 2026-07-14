using Application.Services.SignalR;
using Domain.Contracts.Interfaces;
using Domain.Entities.Backups;

namespace Application.TaskJobs.WorkItems;

internal sealed class BackupRepositoryNotificationWorkItem(
    IBackupRepositoryStreamManager streamManager,
    BackupRepository repository,
    string action = "update") : INotificationWorkItem
{
    public Task ExecuteAsync(CancellationToken cancellationToken)
        => streamManager.SendBackupRepositoryInfo(repository, action);
}

internal sealed class BackupPolicyNotificationWorkItem(
    IBackupPolicyStreamManager streamManager,
    BackupPolicy policy,
    string action = "update") : INotificationWorkItem
{
    public Task ExecuteAsync(CancellationToken cancellationToken)
        => streamManager.SendBackupPolicyInfo(policy, action);
}
