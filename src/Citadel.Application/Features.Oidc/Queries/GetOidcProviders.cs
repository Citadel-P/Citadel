using Application.Features.Oidc.Models;
using Domain;
using Domain.Contracts.Interfaces;
using Hosting.Common;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.Oidc.Queries;

[RequirePermission(ResourceType.Binding, PermissionLevel.Read)]
public sealed record GetOidcProviders : IQuery<Result<OidcProviderListResult>>;

[RequirePermission(ResourceType.Binding, PermissionLevel.Read)]
public sealed record GetOidcProvider(Guid Id) : IQuery<Result<Domain.Entities.Oidc.OidcProvider>>;

public sealed record GetEnabledOidcLoginProviders : IQuery<Result<OidcProviderListResult>>;

internal sealed class GetOidcProvidersHandler(IUnitOfWork unitOfWork)
    : IQueryHandler<GetOidcProviders, Result<OidcProviderListResult>>
{
    public async ValueTask<Result<OidcProviderListResult>> Handle(GetOidcProviders query, CancellationToken cancellationToken)
    {
        var providers = await unitOfWork.OidcProviders.GetAllAsync(cancellationToken);
        return Result.Success(new OidcProviderListResult([.. providers]));
    }
}

internal sealed class GetOidcProviderHandler(IUnitOfWork unitOfWork)
    : IQueryHandler<GetOidcProvider, Result<Domain.Entities.Oidc.OidcProvider>>
{
    public async ValueTask<Result<Domain.Entities.Oidc.OidcProvider>> Handle(GetOidcProvider query, CancellationToken cancellationToken)
    {
        var provider = await unitOfWork.OidcProviders.GetAsync(query.Id, cancellationToken);
        return provider is null
            ? Result.Failure<Domain.Entities.Oidc.OidcProvider>(new NotFoundError("OIDC provider not found."))
            : Result.Success(provider);
    }
}

internal sealed class GetEnabledOidcLoginProvidersHandler(IUnitOfWork unitOfWork)
    : IQueryHandler<GetEnabledOidcLoginProviders, Result<OidcProviderListResult>>
{
    public async ValueTask<Result<OidcProviderListResult>> Handle(GetEnabledOidcLoginProviders query, CancellationToken cancellationToken)
    {
        var providers = await unitOfWork.OidcProviders.GetEnabledAsync(cancellationToken);
        return Result.Success(new OidcProviderListResult([.. providers]));
    }
}
