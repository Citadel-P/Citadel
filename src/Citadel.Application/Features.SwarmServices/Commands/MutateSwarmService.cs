using Application.Services;
using Domain;
using Domain.Contracts.Resources.SwarmServices;
using Hosting.Common;
using Hosting.Common.Abstraction;
using Hosting.Common.Attributes;
using Mediator;
using System.Runtime.CompilerServices;

namespace Application.Features.SwarmServices.Commands;

[RequirePermission(ResourceType.SwarmService, PermissionLevel.Read, SpecificPermission.Apply, ResourceIdProperty = nameof(Id))]
public sealed record ApplySwarmService(Guid Id) : IStreamCommand<SwarmServiceProgressItem>;

[RequirePermission(ResourceType.SwarmService, PermissionLevel.Write, ResourceIdProperty = nameof(Id))]
[RequirePermission(ResourceType.SwarmService, PermissionLevel.Read, SpecificPermission.Apply, ResourceIdProperty = nameof(Id))]
public sealed record ScaleSwarmService(Guid Id, int Replicas) : IStreamCommand<SwarmServiceProgressItem>;

[RequirePermission(ResourceType.SwarmService, PermissionLevel.Read, SpecificPermission.Apply, ResourceIdProperty = nameof(Id))]
public sealed record ForceUpdateSwarmService(Guid Id) : IStreamCommand<SwarmServiceProgressItem>;

[RequirePermission(ResourceType.SwarmService, PermissionLevel.Execute, ResourceIdProperty = nameof(Id))]
public sealed record DeleteSwarmService(Guid Id) : ICommand<LightResults.Result>;

internal sealed class ApplySwarmServiceHandler(
    ISwarmServiceMutationService mutationService,
    IUserContextAccessor userContext) : IStreamCommandHandler<ApplySwarmService, SwarmServiceProgressItem>
{
    public async IAsyncEnumerable<SwarmServiceProgressItem> Handle(
        ApplySwarmService command,
        [EnumeratorCancellation] CancellationToken cancellationToken)
    {
        yield return new(command.Id, null, "validation", "Validating Service configuration and references.");
        yield return new(command.Id, null, "bindings", "Resolving Service variables and secrets.");

        var bindingMessage = new TaskCompletionSource<string>(TaskCreationOptions.RunContinuationsAsynchronously);
        var mutation = mutationService.ApplyAsync(
            command.Id,
            userContext.Current.ActorId,
            cancellationToken,
            message => bindingMessage.TrySetResult(message));
        var firstCompleted = await Task.WhenAny(mutation, bindingMessage.Task);
        if (firstCompleted == bindingMessage.Task || bindingMessage.Task.IsCompletedSuccessfully)
            yield return new(command.Id, null, "bindings", await bindingMessage.Task);

        var result = await mutation;
        if (result.IsSuccess(out var service, out var error))
        {
            yield return new(command.Id, service.CurrentOperation?.Id, "accepted",
                service.CurrentOperation?.State == SwarmServiceOperationState.Completed
                    ? "Docker accepted the Service and reconciliation observed it."
                    : "Docker accepted the Service; rollout observation continues asynchronously.",
                IsCompleted: true,
                IsWarning: service.CurrentOperation?.Warnings?.Count > 0);
        }
        else
        {
            yield return new(command.Id, null, "failed", "Service Apply failed.", IsCompleted: true, ErrorMessage: error!.Message);
        }
    }
}

internal sealed class ScaleSwarmServiceHandler(
    ISwarmServiceMutationService mutationService,
    IUserContextAccessor userContext) : IStreamCommandHandler<ScaleSwarmService, SwarmServiceProgressItem>
{
    public async IAsyncEnumerable<SwarmServiceProgressItem> Handle(
        ScaleSwarmService command,
        [EnumeratorCancellation] CancellationToken cancellationToken)
    {
        yield return new(command.Id, null, "validation", $"Validating scale to {command.Replicas} replicas.");
        var result = await mutationService.ScaleAsync(command.Id, command.Replicas, userContext.Current.ActorId, cancellationToken);
        if (result.IsSuccess(out var service, out var error))
            yield return new(command.Id, service.CurrentOperation?.Id, "accepted", "Docker accepted the replica update.", IsCompleted: true);
        else
            yield return new(command.Id, null, "failed", "Scale failed.", IsCompleted: true, ErrorMessage: error!.Message);
    }
}

internal sealed class ForceUpdateSwarmServiceHandler(
    ISwarmServiceMutationService mutationService,
    IUserContextAccessor userContext) : IStreamCommandHandler<ForceUpdateSwarmService, SwarmServiceProgressItem>
{
    public async IAsyncEnumerable<SwarmServiceProgressItem> Handle(
        ForceUpdateSwarmService command,
        [EnumeratorCancellation] CancellationToken cancellationToken)
    {
        yield return new(command.Id, null, "validation", "Validating task restart.");
        var result = await mutationService.ForceUpdateAsync(command.Id, userContext.Current.ActorId, cancellationToken);
        if (result.IsSuccess(out var service, out var error))
            yield return new(command.Id, service.CurrentOperation?.Id, "accepted", "Docker accepted the task restart.", IsCompleted: true);
        else
            yield return new(command.Id, null, "failed", "Task restart failed.", IsCompleted: true, ErrorMessage: error!.Message);
    }
}

internal sealed class DeleteSwarmServiceHandler(
    ISwarmServiceMutationService mutationService,
    IUserContextAccessor userContext) : ICommandHandler<DeleteSwarmService, LightResults.Result>
{
    public async ValueTask<LightResults.Result> Handle(DeleteSwarmService command, CancellationToken cancellationToken) =>
        await mutationService.DeleteAsync(command.Id, userContext.Current.ActorId, cancellationToken);
}
