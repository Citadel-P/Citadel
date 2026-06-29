using Application.Features.Configuration.Models;
using Application.Features.Configuration;
using Domain.Contracts.Interfaces;
using Domain.Entities.Configuration;
using Hosting.Common;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.Configuration.Queries;

[RequirePermission(ResourceType.Configuration, PermissionLevel.Read)]
public sealed record GetGlobalConfigurationEntries : IQuery<Result<ConfigurationEntriesResult>>;

[RequirePermission(ResourceType.Stack, PermissionLevel.Read, SpecificPermission.Configuration)]
public sealed record GetStackConfigurationEntries(Guid Id) : IQuery<Result<ConfigurationEntriesResult>>;

[RequirePermission(ResourceType.Deployment, PermissionLevel.Read, SpecificPermission.Configuration)]
public sealed record GetDeploymentConfigurationEntries(Guid Id) : IQuery<Result<ConfigurationEntriesResult>>;

internal sealed class GetGlobalConfigurationEntriesHandler(IUnitOfWork unitOfWork)
    : IQueryHandler<GetGlobalConfigurationEntries, Result<ConfigurationEntriesResult>>
{
    public async ValueTask<Result<ConfigurationEntriesResult>> Handle(GetGlobalConfigurationEntries query, CancellationToken cancellationToken)
    {
        var entries = await unitOfWork.ConfigurationEntries.GetEntriesAsync(ConfigurationScope.Global, null, cancellationToken);
        return new ConfigurationEntriesResult([.. entries], [.. entries]);
    }
}

internal sealed class GetStackConfigurationEntriesHandler(IUnitOfWork unitOfWork)
    : IQueryHandler<GetStackConfigurationEntries, Result<ConfigurationEntriesResult>>
{
    public async ValueTask<Result<ConfigurationEntriesResult>> Handle(GetStackConfigurationEntries query, CancellationToken cancellationToken)
    {
        return await ConfigurationEntriesFeatureHelpers.GetResourceEntriesAsync(
            unitOfWork,
            ConfigurationScope.Stack,
            query.Id,
            async () => await unitOfWork.Stacks.GetAsync(query.Id, cancellationToken) is not null,
            cancellationToken);
    }
}

internal sealed class GetDeploymentConfigurationEntriesHandler(IUnitOfWork unitOfWork)
    : IQueryHandler<GetDeploymentConfigurationEntries, Result<ConfigurationEntriesResult>>
{
    public async ValueTask<Result<ConfigurationEntriesResult>> Handle(GetDeploymentConfigurationEntries query, CancellationToken cancellationToken)
    {
        return await ConfigurationEntriesFeatureHelpers.GetResourceEntriesAsync(
            unitOfWork,
            ConfigurationScope.Deployment,
            query.Id,
            async () => await unitOfWork.Deployments.GetAsync(query.Id, cancellationToken) is not null,
            cancellationToken);
    }
}
