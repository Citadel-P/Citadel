using Application.Services.Abstractions;
using Application.Services.SignalR.Context;
using Domain.Entities.Backups;
using static Hosting.Common.Constants;

namespace Application.Services.SignalR;

public interface IBackupRepositoryStreamManager : IStreamGroupManager
{
    Task SendBackupRepositoryInfo(BackupRepository repository, string action = "update");
}

public interface IBackupPolicyStreamManager : IStreamGroupManager
{
    Task SendBackupPolicyInfo(BackupPolicy policy, string action = "update");
}

public interface IBackupRunStreamManager : IStreamGroupManager
{
    Task SendBackupRunInfo(BackupRun run, string action = "update");
}

public interface IBackupRestoreRunStreamManager : IStreamGroupManager
{
    Task SendBackupRestoreRunInfo(BackupRestoreRun run, Guid backupPolicyId, string action = "update");
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

internal sealed class BackupRunStreamManager(IApplicationHubDispatcher dispatcher)
    : BaseStreamManager<StreamContext>, IBackupRunStreamManager
{
    public Task SendBackupRunInfo(BackupRun run, string action = "update")
    {
        if (streams.IsEmpty)
        {
            return Task.CompletedTask;
        }

        return dispatcher.SendBackupRunInfo(run, action);
    }
}

internal sealed class BackupRestoreRunStreamManager(IApplicationHubDispatcher dispatcher)
    : BaseStreamManager<StreamContext>, IBackupRestoreRunStreamManager
{
    public Task SendBackupRestoreRunInfo(BackupRestoreRun run, Guid backupPolicyId, string action = "update")
    {
        if (!streams.ContainsKey(WellKnownSignalRGroups.BackupRestoreRunGroup(run.Id)) &&
            !streams.ContainsKey(WellKnownSignalRGroups.BackupRestoreRunsGroup(backupPolicyId)))
        {
            return Task.CompletedTask;
        }

        return dispatcher.SendBackupRestoreRunInfo(run, backupPolicyId, action);
    }
}
