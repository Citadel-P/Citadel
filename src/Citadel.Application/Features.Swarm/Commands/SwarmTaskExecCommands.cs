using Application.Features.Swarm.Queries;
using Application.Services.SignalR;
using Domain;
using Domain.Contracts.Interfaces;
using Hosting.Common;
using Hosting.Common.Attributes;
using LightResults;
using Mediator;

namespace Application.Features.Swarm.Commands;

[RequirePermission(ResourceType.Platform, PermissionLevel.Read, SpecificPermission.Terminal, ResourceIdProperty = nameof(PlatformId))]
public sealed record StartSwarmTaskShellSession(
    Guid PlatformId,
    string TaskId,
    string SessionId,
    string Shell) : ICommand<Result>;

[RequirePermission(ResourceType.Platform, PermissionLevel.Read, SpecificPermission.Terminal, ResourceIdProperty = nameof(PlatformId))]
public sealed record ResizeSwarmTaskExecSession(
    Guid PlatformId,
    string TaskId,
    string SessionId,
    int Cols,
    int Rows) : ICommand<Result>;

[RequirePermission(ResourceType.Platform, PermissionLevel.Read, SpecificPermission.Terminal, ResourceIdProperty = nameof(PlatformId))]
public sealed record SendSwarmTaskExecInput(
    Guid PlatformId,
    string TaskId,
    string SessionId,
    byte[] Data) : ICommand<Result>;

internal sealed class StartSwarmTaskShellSessionHandler(
    IUnitOfWork unitOfWork,
    IConnectorFactory<ISwarmConnector> swarmConnectorFactory,
    IExecSessionManager execSessionManager)
    : ICommandHandler<StartSwarmTaskShellSession, Result>
{
    public async ValueTask<Result> Handle(StartSwarmTaskShellSession command, CancellationToken cancellationToken)
    {
        var resolved = await SwarmTaskRuntimeQuery.LoadRunningTargetAsync(
            unitOfWork,
            swarmConnectorFactory,
            command.PlatformId,
            command.TaskId,
            cancellationToken);
        if (!resolved.IsSuccess(out var target, out var error))
            return Result.Failure(error!);

        await execSessionManager.StartSwarmTaskExecProcess(
            target.Platform,
            command.TaskId,
            target.DockerNodeId,
            target.DockerContainerId,
            command.SessionId,
            command.Shell,
            cancellationToken);
        return Result.Success();
    }
}

internal sealed class ResizeSwarmTaskExecSessionHandler(IExecSessionManager execSessionManager)
    : ICommandHandler<ResizeSwarmTaskExecSession, Result>
{
    public async ValueTask<Result> Handle(ResizeSwarmTaskExecSession command, CancellationToken cancellationToken)
    {
        await execSessionManager.ResizeSwarmTaskAsync(
            command.PlatformId,
            command.TaskId,
            command.SessionId,
            command.Cols,
            command.Rows,
            cancellationToken);
        return Result.Success();
    }
}

internal sealed class SendSwarmTaskExecInputHandler(IExecSessionManager execSessionManager)
    : ICommandHandler<SendSwarmTaskExecInput, Result>
{
    public async ValueTask<Result> Handle(SendSwarmTaskExecInput command, CancellationToken cancellationToken)
    {
        await execSessionManager.SendSwarmTaskInputAsync(
            command.PlatformId,
            command.TaskId,
            command.SessionId,
            command.Data,
            cancellationToken);
        return Result.Success();
    }
}
