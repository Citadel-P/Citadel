using Domain.Contracts.Interfaces;
using Domain.Entities;
using FluentValidation;
using Hosting.Common.Attributes;
using LightResults;
using Mediator;
using Hosting.Common;

namespace Application.Features.Platforms.Queries;

[RequirePermission(ResourceType.Platform, PermissionLevel.Read)]
public sealed record class GetContainers(Guid PlatformId) : IQuery<Result<IEnumerable<Container>>>
{
    internal class Validator : AbstractValidator<GetContainers>
    {
        public Validator()
        {
            RuleFor(s => s.PlatformId).NotNull();
        }
    }
}

internal class GetContainersHandler(IUnitOfWork unitOfWork) : IQueryHandler<GetContainers, Result<IEnumerable<Container>>>
{
    public async ValueTask<Result<IEnumerable<Container>>> Handle(GetContainers query, CancellationToken cancellationToken)
    {
        var containers = await unitOfWork.Containers.GetContainersInfoAsync(query.PlatformId, cancellationToken);
        return Result.Success(containers ?? []);
    }
}