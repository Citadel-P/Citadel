using Domain.Contracts.Interfaces;
using Domain.Entities.Configuration;
using Hosting.Common;
using Hosting.Common.Attributes;
using LightResults;
using Mediator;

namespace Application.Features.Configuration.Queries;

[RequirePermission(ResourceType.Configuration, PermissionLevel.Read)]
public sealed record GetSecretProviders : IQuery<Result<IReadOnlyList<SecretProvider>>>;

internal sealed class GetSecretProvidersHandler(IUnitOfWork unitOfWork)
    : IQueryHandler<GetSecretProviders, Result<IReadOnlyList<SecretProvider>>>
{
    public async ValueTask<Result<IReadOnlyList<SecretProvider>>> Handle(GetSecretProviders query, CancellationToken cancellationToken)
    {
        var providers = await unitOfWork.SecretProviders.GetAllAsync(cancellationToken);
        return Result.Success<IReadOnlyList<SecretProvider>>([.. providers]);
    }
}
