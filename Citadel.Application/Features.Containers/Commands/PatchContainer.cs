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

internal class PatchContainerHandler(IConnectorFactory<IContainerConnector> connectorFactory, ApplicationDbContext dbContext)
    : ICommandHandler<PatchContainer, Result>
{
    public async ValueTask<Result> Handle(PatchContainer request, CancellationToken cancellationToken)
    {
        var platformContainers = await dbContext.Containers.AsNoTracking().Include(s => s.Platform)
                        .Where(s => request.ContainersIds.Contains(s.ContainerId))
                        .GroupBy(s => new { s.Platform.Address, s.Platform.Id })
                        .Select(s => new { s.Key, Containers = s.Select(x => x.ContainerId) })
                        .ToDictionaryAsync(s => s.Key, s => s.Containers, cancellationToken);

        if (platformContainers.Count == 0)
        {
            return Result.Failure(new NotFoundError("No platform found for the given IDs."));
        }

        foreach (var platform in platformContainers)
        {
            var command = new PatchContainerCommand
            (
                PlatformAddress: platform.Key.Address,
                ContainerIds: platform.Value,
                Action: request.Action
            );
            var connector = connectorFactory.GetConnector(platform.Key.Id);
            if (connector is not null)
            {
                await connector.PatchAsync(command, cancellationToken);
            }
            else
            {
                return Result.Failure(new NotFoundError($"No connector found for platform {platform.Key.Address}."));
            }
        }

        return Result.Success();
    }
}