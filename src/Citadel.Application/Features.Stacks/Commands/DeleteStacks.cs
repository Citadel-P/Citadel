using Application.Services;
using Application.Services.SignalR;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Containers;
using Domain.Entities.Stacks;
using Hosting.Common;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.Stacks.Commands;

[RequirePermission(ResourceType.Stack, PermissionLevel.Execute)]
public sealed record DeleteStacks(IEnumerable<Guid> Ids) : ICommand<Result>;

internal sealed class DeleteStacksHandler(
    IUnitOfWork unitOfWork,
    IPlatformContainerCache platformCache,
    IConnectorFactory<IContainerConnector> connectorFactory,
    IStackStreamManager stackHub) : ICommandHandler<DeleteStacks, Result>
{
    public async ValueTask<Result> Handle(DeleteStacks command, CancellationToken cancellationToken)
    {
        var stacks = (await unitOfWork.Stacks.GetAllAsync(command.Ids, cancellationToken))?.ToArray();
        if (stacks == null || stacks.Length == 0)
        {
            return Result.Failure(new NotFoundError("No stacks found matching the provided IDs."));
        }

        foreach (var stack in stacks)
        {
            var cleanupResult = await DeleteRuntimeContainersAsync(stack, cancellationToken);
            if (cleanupResult.IsFailure())
            {
                return cleanupResult;
            }
        }

        await unitOfWork.Stacks.RemoveRangeAsync(stacks.Select(stack => stack.Id), cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);

        foreach (var stack in stacks)
        {
            await stackHub.SendStackInfo(stack, "delete");
        }

        return Result.Success();
    }

    private async Task<Result> DeleteRuntimeContainersAsync(Stack stack, CancellationToken cancellationToken)
    {
        if (stack.CurrentStackRelease is null || stack.CurrentStackRelease.Status == StackReleaseStatus.Created)
        {
            return Result.Success();
        }

        if (!platformCache.TryGetCacheEntry(stack.CurrentStackRelease.PlatformId, out var platform, out var platformError))
        {
            return Result.Failure(new NotFoundError(platformError?.Message ?? "Platform not found or disconnected."));
        }

        string projectName;
        try
        {
            projectName = StackProjectNameResolver.Resolve(stack);
        }
        catch (InvalidOperationException ex)
        {
            return Result.Failure(new BadRequestError(ex.Message));
        }

        var connector = connectorFactory.GetConnector(platform.ConnectorType);
        var listResult = await connector.ListContainersAsync(
            new ContainerFilterCommand(
                PlatformAddress: platform.Address,
                All: true,
                Filters: new Dictionary<string, IDictionary<string, bool>>
                {
                    ["label"] = new Dictionary<string, bool>
                    {
                        [$"{ComposeLabels.Project}={projectName}"] = true
                    }
                }),
            cancellationToken);

        if (listResult.IsFailure(out var listError, out var containers))
        {
            return Result.Failure(new BadRequestError(listError.Message));
        }

        var ownedContainerIds = new List<string>();
        foreach (var container in containers.Values)
        {
            var inspectResult = await connector.InspectAsync(
                new InspectContainerCommand(platform.Address, container.Id),
                cancellationToken);

            if (inspectResult.IsFailure(out var inspectError, out var inspect))
            {
                return Result.Failure(new BadRequestError(
                    $"Unable to inspect stack container '{container.Name}' ({container.Id}): {inspectError.Message}"));
            }

            var labels = inspect.Config?.Labels ?? new Dictionary<string, string>();
            if (StackContainerOwnership.IsOwnedByStack(labels, stack.Id))
            {
                ownedContainerIds.Add(container.Id);
            }
        }

        if (ownedContainerIds.Count == 0)
        {
            return Result.Success();
        }

        return await connector.DeleteAsync(
            new DeleteContainerCommand(
                ownedContainerIds.Distinct(StringComparer.OrdinalIgnoreCase),
                platform.Address,
                Volume: false,
                Force: true,
                Link: false),
            cancellationToken);
    }
}
