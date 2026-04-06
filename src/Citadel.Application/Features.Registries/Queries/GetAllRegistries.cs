using Hosting.Common;
using Domain.Contracts.Interfaces;
using Domain.Entities.Registries;
using Hosting.Common.Extensions;
using LightResults;
using Mediator;
using Domain;
using Microsoft.AspNetCore.Http;

namespace Application.Features.Registries.Queries;

public sealed record GetAllRegistries(bool? IncludeDisabled) : IQuery<Result<IEnumerable<Registry>>>;

internal sealed class GetAllRegistriesHandler(IUnitOfWork unitOfWork, IHttpContextAccessor httpContextAccessor) : IQueryHandler<GetAllRegistries, Result<IEnumerable<Registry>>>
{
    public async ValueTask<Result<IEnumerable<Registry>>> Handle(GetAllRegistries query, CancellationToken cancellationToken)
    {
        var user = httpContextAccessor.HttpContext?.User;
        var registries = user is not null && !user.IsAdmin()
            ? await unitOfWork.Registries.GetAuthorizedAsync(user.GetUserId(), ResourceType.Registry, ResourceAction.View, cancellationToken)
            : await unitOfWork.Registries.GetAllAsync(cancellationToken);

        if (query.IncludeDisabled == true)
        {
            return Result.Success<IEnumerable<Registry>>(registries.OrderByDescending(s => s.CreatedAt));
        }
        else
        {
            return Result.Success(registries.OrderByDescending(s => s.CreatedAt).Where(r => r.Status != RegistryStatus.Disabled));
        }
    }
}
