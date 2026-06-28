using Domain.Contracts.Interfaces;
using Domain.Entities.Configuration;
using Hosting.Common;
using Hosting.Common.Attributes;
using LightResults;
using Mediator;

namespace Application.Features.Configuration.Queries;

[RequirePermission(ResourceType.Configuration, PermissionLevel.Read)]
public sealed record GetSecretDefinitions : IQuery<Result<IReadOnlyList<SecretDefinition>>>;

internal sealed class GetSecretDefinitionsHandler(IUnitOfWork unitOfWork)
    : IQueryHandler<GetSecretDefinitions, Result<IReadOnlyList<SecretDefinition>>>
{
    public async ValueTask<Result<IReadOnlyList<SecretDefinition>>> Handle(GetSecretDefinitions query, CancellationToken cancellationToken)
    {
        var secrets = await unitOfWork.SecretDefinitions.GetAllAsync(cancellationToken);
        return Result.Success<IReadOnlyList<SecretDefinition>>([.. secrets]);
    }
}
