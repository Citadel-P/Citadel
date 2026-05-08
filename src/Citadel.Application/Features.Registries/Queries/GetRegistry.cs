using Hosting.Common;
using Domain.Contracts.Interfaces;
using Domain.Entities.Registries;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.Registries.Queries;

[RequirePermission(ResourceType.Registry, PermissionLevel.Read)]
public sealed record GetRegistry(Guid Id) : IQuery<Result<Registry>>;
internal sealed class GetRegistryHandler(IUnitOfWork unitOfWork) : IQueryHandler<GetRegistry, Result<Registry>>
{
    public async ValueTask<Result<Registry>> Handle(GetRegistry query, CancellationToken cancellationToken)
    {
        var registry = await unitOfWork.Registries.GetAsync(query.Id,  cancellationToken);
        return registry ?? Result.Failure<Registry>(new NotFoundError($"Registry with ID {query.Id} does not exist"));
    }
}