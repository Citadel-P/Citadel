using Application.Services.SignalR;
using Domain.Contracts.Interfaces;
using Hosting.Common;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.Stacks.Commands;

[RequirePermission(ResourceType.Stack, PermissionLevel.Read, SpecificPermission.Terminal)]
public sealed record StartStackShellSession(Guid Id, string ContainerId, string SessionId, string Shell) : ICommand<Result>;

[RequirePermission(ResourceType.Stack, PermissionLevel.Read, SpecificPermission.Terminal)]
public sealed record ResizeStackExecSession(Guid Id, string ContainerId, string SessionId, int Cols, int Rows) : ICommand<Result>;

[RequirePermission(ResourceType.Stack, PermissionLevel.Read, SpecificPermission.Terminal)]
public sealed record SendStackExecInput(Guid Id, string ContainerId, string SessionId, byte[] Data) : ICommand<Result>;

internal sealed class StartStackShellSessionHandler(
    IUnitOfWork unitOfWork,
    IExecSessionManager execSessionManager)
    : ICommandHandler<StartStackShellSession, Result>
{
    public async ValueTask<Result> Handle(StartStackShellSession command, CancellationToken cancellationToken)
    {
        var containerId = await StackContainerResolver.ResolveContainerId(
            unitOfWork,
            command.Id,
            command.ContainerId,
            cancellationToken);
        if (containerId is null)
        {
            return Result.Failure(new NotFoundError("Container does not exist in this stack"));
        }

        await execSessionManager.StartExecProcess(containerId, command.SessionId, command.Shell, cancellationToken);
        return Result.Success();
    }
}

internal sealed class ResizeStackExecSessionHandler(
    IUnitOfWork unitOfWork,
    IExecSessionManager execSessionManager)
    : ICommandHandler<ResizeStackExecSession, Result>
{
    public async ValueTask<Result> Handle(ResizeStackExecSession command, CancellationToken cancellationToken)
    {
        var containerId = await StackContainerResolver.ResolveContainerId(
            unitOfWork,
            command.Id,
            command.ContainerId,
            cancellationToken);
        if (containerId is null)
        {
            return Result.Failure(new NotFoundError("Container does not exist in this stack"));
        }

        await execSessionManager.ResizeAsync(containerId, command.SessionId, command.Cols, command.Rows, cancellationToken);
        return Result.Success();
    }
}

internal sealed class SendStackExecInputHandler(
    IUnitOfWork unitOfWork,
    IExecSessionManager execSessionManager)
    : ICommandHandler<SendStackExecInput, Result>
{
    public async ValueTask<Result> Handle(SendStackExecInput command, CancellationToken cancellationToken)
    {
        var containerId = await StackContainerResolver.ResolveContainerId(
            unitOfWork,
            command.Id,
            command.ContainerId,
            cancellationToken);
        if (containerId is null)
        {
            return Result.Failure(new NotFoundError("Container does not exist in this stack"));
        }

        await execSessionManager.SendInputAsync(containerId, command.SessionId, command.Data, cancellationToken);
        return Result.Success();
    }
}

file static class StackContainerResolver
{
    public static async Task<string?> ResolveContainerId(
        IUnitOfWork unitOfWork,
        Guid stackId,
        string containerId,
        CancellationToken cancellationToken)
    {
        var containers = await unitOfWork.Stacks.GetContainersAsync(stackId, cancellationToken);
        return containers.FirstOrDefault(x => x.DockerContainerId.StartsWith(containerId))?.DockerContainerId;
    }
}
