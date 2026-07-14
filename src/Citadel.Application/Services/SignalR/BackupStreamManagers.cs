using Application.Services.Abstractions;
using Application.Services.SignalR.Context;
using Domain.Entities.Backups;

namespace Application.Services.SignalR;

public interface IBackupRepositoryStreamManager : IStreamGroupManager
{
    Task SendBackupRepositoryInfo(BackupRepository repository, string action = "update");
}

public interface IBackupPolicyStreamManager : IStreamGroupManager
{
    Task SendBackupPolicyInfo(BackupPolicy policy, string action = "update");
}

internal sealed class BackupRepositoryStreamManager(IApplicationHubDispatcher dispatcher)
    : BaseStreamManager<StreamContext>, IBackupRepositoryStreamManager
{
    public Task SendBackupRepositoryInfo(BackupRepository repository, string action = "update")
    {
        if (streams.IsEmpty)
        {
            return Task.CompletedTask;
        }

        return dispatcher.SendBackupRepositoryInfo(repository, action);
    }
}

internal sealed class BackupPolicyStreamManager(IApplicationHubDispatcher dispatcher)
    : BaseStreamManager<StreamContext>, IBackupPolicyStreamManager
{
    public Task SendBackupPolicyInfo(BackupPolicy policy, string action = "update")
    {
        if (streams.IsEmpty)
        {
            return Task.CompletedTask;
        }

        return dispatcher.SendBackupPolicyInfo(policy, action);
    }
}
