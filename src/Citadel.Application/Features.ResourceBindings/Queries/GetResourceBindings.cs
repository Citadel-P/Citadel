using Application.Features.ResourceBindings.Models;
using Domain.Contracts.Interfaces;
using Hosting.Common;
using Hosting.Common.Attributes;
using LightResults;
using Mediator;
using Domain;

namespace Application.Features.ResourceBindings.Queries;

[RequirePermission(ResourceType.Binding, PermissionLevel.Read)]
public sealed record GetGlobalResourceBindings : IQuery<Result<ResourceBindingsResult>>;

[RequirePermission(ResourceType.Stack, PermissionLevel.Read, SpecificPermission.ResourceBindings)]
public sealed record GetStackResourceBindings(Guid Id) : IQuery<Result<ResourceBindingsResult>>;

[RequirePermission(ResourceType.Deployment, PermissionLevel.Read, SpecificPermission.ResourceBindings)]
public sealed record GetDeploymentResourceBindings(Guid Id) : IQuery<Result<ResourceBindingsResult>>;

internal sealed class GetGlobalResourceBindingsHandler(IUnitOfWork unitOfWork)
    : IQueryHandler<GetGlobalResourceBindings, Result<ResourceBindingsResult>>
{
    public async ValueTask<Result<ResourceBindingsResult>> Handle(GetGlobalResourceBindings query, CancellationToken cancellationToken)
    {
        var entries = await unitOfWork.ResourceBindings.GetEntriesAsync(ResourceBindingScope.Global, null, cancellationToken);
        return new ResourceBindingsResult([.. entries], [.. entries]);
    }
}

internal sealed class GetStackResourceBindingsHandler(IUnitOfWork unitOfWork)
    : IQueryHandler<GetStackResourceBindings, Result<ResourceBindingsResult>>
{
    public async ValueTask<Result<ResourceBindingsResult>> Handle(GetStackResourceBindings query, CancellationToken cancellationToken)
    {
        return await ResourceBindingsFeatureHelpers.GetResourceEntriesAsync(
            unitOfWork,
            ResourceBindingScope.Stack,
            query.Id,
            async () => await unitOfWork.Stacks.GetAsync(query.Id, cancellationToken) is not null,
            cancellationToken);
    }
}

internal sealed class GetDeploymentResourceBindingsHandler(IUnitOfWork unitOfWork)
    : IQueryHandler<GetDeploymentResourceBindings, Result<ResourceBindingsResult>>
{
    public async ValueTask<Result<ResourceBindingsResult>> Handle(GetDeploymentResourceBindings query, CancellationToken cancellationToken)
    {
        return await ResourceBindingsFeatureHelpers.GetResourceEntriesAsync(
            unitOfWork,
            ResourceBindingScope.Deployment,
            query.Id,
            async () => await unitOfWork.Deployments.GetAsync(query.Id, cancellationToken) is not null,
            cancellationToken);
    }
}
