using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.ResourceBindings;
using Hosting.Common;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.ResourceBindings.Queries;

[RequirePermission(ResourceType.Binding, PermissionLevel.Read)]
public sealed record GetSecretDefinitions : IQuery<Result<IReadOnlyList<SecretDefinition>>>;

[RequirePermission(ResourceType.Stack, PermissionLevel.Read, SpecificPermission.ResourceBindings)]
public sealed record GetStackSecretDefinitions(Guid Id) : IQuery<Result<IReadOnlyList<SecretDefinition>>>;

[RequirePermission(ResourceType.Deployment, PermissionLevel.Read, SpecificPermission.ResourceBindings)]
public sealed record GetDeploymentSecretDefinitions(Guid Id) : IQuery<Result<IReadOnlyList<SecretDefinition>>>;

internal sealed class GetSecretDefinitionsHandler(IUnitOfWork unitOfWork)
    : IQueryHandler<GetSecretDefinitions, Result<IReadOnlyList<SecretDefinition>>>
{
    public async ValueTask<Result<IReadOnlyList<SecretDefinition>>> Handle(GetSecretDefinitions query, CancellationToken cancellationToken)
    {
        var secrets = await unitOfWork.SecretDefinitions.GetBoundAsync(ResourceBindingScope.Global, null, cancellationToken);
        return Result.Success<IReadOnlyList<SecretDefinition>>([.. secrets]);
    }
}

internal sealed class GetStackSecretDefinitionsHandler(IUnitOfWork unitOfWork)
    : IQueryHandler<GetStackSecretDefinitions, Result<IReadOnlyList<SecretDefinition>>>
{
    public async ValueTask<Result<IReadOnlyList<SecretDefinition>>> Handle(GetStackSecretDefinitions query, CancellationToken cancellationToken)
    {
        var stack = await unitOfWork.Stacks.GetAsync(query.Id, cancellationToken);
        if (stack is null)
            return Result.Failure<IReadOnlyList<SecretDefinition>>(new NotFoundError($"Stack with ID {query.Id} does not exist."));

        var secrets = await unitOfWork.SecretDefinitions.GetBoundAsync(ResourceBindingScope.Stack, query.Id, cancellationToken);
        return Result.Success<IReadOnlyList<SecretDefinition>>([.. secrets]);
    }
}

internal sealed class GetDeploymentSecretDefinitionsHandler(IUnitOfWork unitOfWork)
    : IQueryHandler<GetDeploymentSecretDefinitions, Result<IReadOnlyList<SecretDefinition>>>
{
    public async ValueTask<Result<IReadOnlyList<SecretDefinition>>> Handle(GetDeploymentSecretDefinitions query, CancellationToken cancellationToken)
    {
        var deployment = await unitOfWork.Deployments.GetAsync(query.Id, cancellationToken);
        if (deployment is null)
            return Result.Failure<IReadOnlyList<SecretDefinition>>(new NotFoundError($"Deployment with ID {query.Id} does not exist."));

        var secrets = await unitOfWork.SecretDefinitions.GetBoundAsync(ResourceBindingScope.Deployment, query.Id, cancellationToken);
        return Result.Success<IReadOnlyList<SecretDefinition>>([.. secrets]);
    }
}
