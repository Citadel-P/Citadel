using System.Runtime.CompilerServices;
using Application.Services;
using Domain;
using Domain.Contracts.Resources.Platforms;
using Hosting.Common;
using Hosting.Common.Abstraction;
using Hosting.Common.Attributes;
using Mediator;

namespace Application.Features.Platforms.Commands;

[RequirePermission(ResourceType.Platform, PermissionLevel.Execute, SpecificPermission.ManageNodeAgents, ResourceIdProperty = nameof(PlatformId))]
public sealed record InstallSwarmNodeAgents(Guid PlatformId, string CoreUrl) : IStreamCommand<SwarmNodeAgentProgressItem>;

[RequirePermission(ResourceType.Platform, PermissionLevel.Execute, SpecificPermission.ManageNodeAgents, ResourceIdProperty = nameof(PlatformId))]
public sealed record RepairSwarmNodeAgents(Guid PlatformId, string CoreUrl) : IStreamCommand<SwarmNodeAgentProgressItem>;

[RequirePermission(ResourceType.Platform, PermissionLevel.Execute, SpecificPermission.ManageNodeAgents, ResourceIdProperty = nameof(PlatformId))]
public sealed record UpgradeSwarmNodeAgents(Guid PlatformId, string CoreUrl) : IStreamCommand<SwarmNodeAgentProgressItem>;

[RequirePermission(ResourceType.Platform, PermissionLevel.Execute, SpecificPermission.ManageNodeAgents, ResourceIdProperty = nameof(PlatformId))]
public sealed record RemoveSwarmNodeAgents(Guid PlatformId) : IStreamCommand<SwarmNodeAgentProgressItem>;

internal abstract class SwarmNodeAgentCommandHandler(
    ISwarmNodeAgentLifecycleService lifecycleService,
    IUserContextAccessor userContext)
{
    protected async IAsyncEnumerable<SwarmNodeAgentProgressItem> Execute(
        Guid platformId,
        SwarmNodeAgentOperationKind kind,
        string coreUrl,
        [EnumeratorCancellation] CancellationToken cancellationToken)
    {
        var messages = System.Threading.Channels.Channel.CreateBounded<SwarmNodeAgentProgressItem>(
            Hosting.Common.Helpers.ChannelDefaultOptions(32, singleWriter: true, singleReader: true));
        var execution = lifecycleService.ExecuteAsync(
            platformId,
            kind,
            coreUrl,
            userContext.Current.ActorId,
            item => messages.Writer.TryWrite(item),
            cancellationToken);
        _ = execution.ContinueWith(
            static (_, state) => ((System.Threading.Channels.ChannelWriter<SwarmNodeAgentProgressItem>)state!).TryComplete(),
            messages.Writer,
            CancellationToken.None,
            TaskContinuationOptions.ExecuteSynchronously,
            TaskScheduler.Default);

        while (!execution.IsCompleted)
        {
            while (messages.Reader.TryRead(out var message))
                yield return message;
            if (!execution.IsCompleted)
                await messages.Reader.WaitToReadAsync(cancellationToken);
        }
        while (messages.Reader.TryRead(out var message))
            yield return message;

        var result = await execution;
        if (result.IsSuccess(out var operationId, out var error))
            yield return new(platformId, operationId, "completed", $"Node-agent {kind.ToString().ToLowerInvariant()} completed.", IsCompleted: true);
        else
            yield return new(platformId, Guid.Empty, "failed", $"Node-agent {kind.ToString().ToLowerInvariant()} failed.", IsCompleted: true, ErrorMessage: error!.Message);
    }
}

internal sealed class InstallSwarmNodeAgentsHandler(ISwarmNodeAgentLifecycleService service, IUserContextAccessor userContext)
    : SwarmNodeAgentCommandHandler(service, userContext), IStreamCommandHandler<InstallSwarmNodeAgents, SwarmNodeAgentProgressItem>
{
    public IAsyncEnumerable<SwarmNodeAgentProgressItem> Handle(InstallSwarmNodeAgents command, CancellationToken cancellationToken)
        => Execute(command.PlatformId, SwarmNodeAgentOperationKind.Install, command.CoreUrl, cancellationToken);
}

internal sealed class RepairSwarmNodeAgentsHandler(ISwarmNodeAgentLifecycleService service, IUserContextAccessor userContext)
    : SwarmNodeAgentCommandHandler(service, userContext), IStreamCommandHandler<RepairSwarmNodeAgents, SwarmNodeAgentProgressItem>
{
    public IAsyncEnumerable<SwarmNodeAgentProgressItem> Handle(RepairSwarmNodeAgents command, CancellationToken cancellationToken)
        => Execute(command.PlatformId, SwarmNodeAgentOperationKind.Repair, command.CoreUrl, cancellationToken);
}

internal sealed class UpgradeSwarmNodeAgentsHandler(ISwarmNodeAgentLifecycleService service, IUserContextAccessor userContext)
    : SwarmNodeAgentCommandHandler(service, userContext), IStreamCommandHandler<UpgradeSwarmNodeAgents, SwarmNodeAgentProgressItem>
{
    public IAsyncEnumerable<SwarmNodeAgentProgressItem> Handle(UpgradeSwarmNodeAgents command, CancellationToken cancellationToken)
        => Execute(command.PlatformId, SwarmNodeAgentOperationKind.Upgrade, command.CoreUrl, cancellationToken);
}

internal sealed class RemoveSwarmNodeAgentsHandler(ISwarmNodeAgentLifecycleService service, IUserContextAccessor userContext)
    : SwarmNodeAgentCommandHandler(service, userContext), IStreamCommandHandler<RemoveSwarmNodeAgents, SwarmNodeAgentProgressItem>
{
    public IAsyncEnumerable<SwarmNodeAgentProgressItem> Handle(RemoveSwarmNodeAgents command, CancellationToken cancellationToken)
        => Execute(command.PlatformId, SwarmNodeAgentOperationKind.Remove, string.Empty, cancellationToken);
}
