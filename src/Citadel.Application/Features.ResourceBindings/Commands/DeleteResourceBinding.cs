using Application.Features.ResourceBindings.Models;
using Domain;
using Domain.Contracts.Interfaces;
using Hosting.Common;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.ResourceBindings.Commands;

[RequirePermission(ResourceType.Binding, PermissionLevel.Write)]
public sealed record DeleteGlobalResourceBinding(Guid Id) : ICommand<Result<ResourceBindingsResult>>;

[RequirePermission(ResourceType.Stack, PermissionLevel.Write, SpecificPermission.ResourceBindings, ResourceIdProperty = nameof(DeleteStackResourceBinding.ResourceId))]
public sealed record DeleteStackResourceBinding(Guid ResourceId, Guid Id) : ICommand<Result<ResourceBindingsResult>>;

[RequirePermission(ResourceType.Deployment, PermissionLevel.Write, SpecificPermission.ResourceBindings, ResourceIdProperty = nameof(DeleteDeploymentResourceBinding.ResourceId))]
public sealed record DeleteDeploymentResourceBinding(Guid ResourceId, Guid Id) : ICommand<Result<ResourceBindingsResult>>;

internal sealed class DeleteGlobalResourceBindingHandler(IUnitOfWork unitOfWork)
    : ICommandHandler<DeleteGlobalResourceBinding, Result<ResourceBindingsResult>>
{
    public async ValueTask<Result<ResourceBindingsResult>> Handle(DeleteGlobalResourceBinding command, CancellationToken cancellationToken)
        => await ResourceBindingsFeatureHelpers.DeleteEntryAsync(
            unitOfWork,
            ResourceBindingScope.Global,
            null,
            command.Id,
            cancellationToken);
}

internal sealed class DeleteStackResourceBindingHandler(IUnitOfWork unitOfWork)
    : ICommandHandler<DeleteStackResourceBinding, Result<ResourceBindingsResult>>
{
    public async ValueTask<Result<ResourceBindingsResult>> Handle(DeleteStackResourceBinding command, CancellationToken cancellationToken)
    {
        var stack = await unitOfWork.Stacks.GetAsync(command.ResourceId, cancellationToken);
        if (stack is null)
            return Result.Failure<ResourceBindingsResult>(new NotFoundError($"Stack with ID {command.ResourceId} does not exist."));

        return await ResourceBindingsFeatureHelpers.DeleteEntryAsync(
            unitOfWork,
            ResourceBindingScope.Stack,
            command.ResourceId,
            command.Id,
            cancellationToken);
    }
}

internal sealed class DeleteDeploymentResourceBindingHandler(IUnitOfWork unitOfWork)
    : ICommandHandler<DeleteDeploymentResourceBinding, Result<ResourceBindingsResult>>
{
    public async ValueTask<Result<ResourceBindingsResult>> Handle(DeleteDeploymentResourceBinding command, CancellationToken cancellationToken)
    {
        var deployment = await unitOfWork.Deployments.GetAsync(command.ResourceId, cancellationToken);
        if (deployment is null)
            return Result.Failure<ResourceBindingsResult>(new NotFoundError($"Deployment with ID {command.ResourceId} does not exist."));

        return await ResourceBindingsFeatureHelpers.DeleteEntryAsync(
            unitOfWork,
            ResourceBindingScope.Deployment,
            command.ResourceId,
            command.Id,
            cancellationToken);
    }
}
