using Domain.Contracts.Interfaces;
using Domain.Entities;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.Attributes;
using LightResults;
using Mediator;

namespace Application.Features.Stacks.Queries;

[RequirePermission(ResourceType.Stack, PermissionLevel.Read)]
public sealed record GetStackStats(Guid Id, int Hours = 24) : IQuery<Result<IEnumerable<StackContainerStats>>>
{
    internal sealed class Validator : AbstractValidator<GetStackStats>
    {
        public Validator()
            => RuleFor(s => s.Hours)
                .Must(hours => hours is 24 or 48 or 72)
                .WithMessage("Hours must be one of: 24, 48, 72.");
    }
}

public sealed record StackContainerStats(string ContainerId, string ContainerName, IEnumerable<ContainerStat> Stats);

internal sealed class GetStackStatsHandler(IUnitOfWork unitOfWork) : IQueryHandler<GetStackStats, Result<IEnumerable<StackContainerStats>>>
{
    public async ValueTask<Result<IEnumerable<StackContainerStats>>> Handle(GetStackStats query, CancellationToken cancellationToken)
    {
        var containers = await unitOfWork.Stacks.GetContainersAsync(query.Id, cancellationToken);
        var result = new List<StackContainerStats>();

        foreach (var container in containers)
        {
            if (string.IsNullOrWhiteSpace(container.DockerContainerId))
            {
                continue;
            }

            var stats = await unitOfWork.ContainerStats.GetStatsAggregatedAsync(
                container.DockerContainerId,
                query.Hours,
                cancellationToken);

            result.Add(new StackContainerStats(container.DockerContainerId, container.Name, stats));
        }

        return Result.Success<IEnumerable<StackContainerStats>>(result);
    }
}
