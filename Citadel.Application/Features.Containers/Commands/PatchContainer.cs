using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Containers;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.ErrorTypes;
using Infrastructure.EntityFramework;
using LightResults;
using Mediator;
using Microsoft.EntityFrameworkCore;

namespace Application.Features.Containers.Commands;

public sealed record PatchContainer(string[] ContainersIds, ContainerAction Action) : ICommand<Result>
{
    internal class Validator : AbstractValidator<PatchContainer>
    {
        public Validator()
            => RuleForEach(s => s.ContainersIds).ValidContainerId();
    }
}

internal class PatchContainerHandler(IContainerConnector containerConnector, ApplicationDbContext dbContext)
    : ICommandHandler<PatchContainer, Result>
{
    public async ValueTask<Result> Handle(PatchContainer request, CancellationToken cancellationToken)
    {
        var platformContainers = await dbContext.Containers.AsNoTracking().Include(s => s.Platform)
                        .Where(s => request.ContainersIds.Contains(s.ContainerId))
                        .GroupBy(s => s.Platform.Address)
                        .Select(s => new KeyValuePair<string, IEnumerable<string>>(s.Key, s.Select(x => x.ContainerId)))
                        .ToDictionaryAsync(s => s.Key, s => s.Value, cancellationToken);

        if (platformContainers.Count == 0)
        {
            return Result.Failure(new NotFoundError("No platform found for the given IDs."));
        }

        var command = new PatchContainerCommand
        (
            Action: request.Action,
            PlatformContainers: platformContainers
        );
        return await containerConnector.PatchAsync(command, cancellationToken);
    }
}