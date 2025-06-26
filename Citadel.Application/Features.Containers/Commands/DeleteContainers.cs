using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Containers;
using Domain.Entities;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;
using Microsoft.EntityFrameworkCore;

namespace Application.Features.Containers.Commands;

public sealed record DeleteContainers(string[] ContainersIds, bool? V = false, bool? Force = false, bool? Link = false) : ICommand<Result>
{
    internal class Validator : AbstractValidator<DeleteContainers>
    {
        public Validator()
            => RuleForEach(s => s.ContainersIds).ValidContainerId();
    }
}

internal sealed class DeleteContainersHandler(IUnitOfWork unitOfWork, IConnectorFactory<IContainerConnector> connectorFactory)
    : ICommandHandler<DeleteContainers, Result>
{
    public async ValueTask<Result> Handle(DeleteContainers request, CancellationToken cancellationToken)
    {
        var platformContainers = await unitOfWork.Containers
                    .Query().AsNoTracking()
                    .Include(s => s.Platform)
                    .Where(s => request.ContainersIds.Contains(s.ContainerId))
                    .GroupBy(s => new { s.Platform.Address, s.Platform.ConnectorType })
                    .Select(g => new
                    {
                        g.Key.Address,
                        g.Key.ConnectorType,
                        ContainersId = g.Select(x => x.ContainerId).ToArray()
                    }).ToListAsync(cancellationToken);

        if (platformContainers.Count == 0)
        {
            return Result.Failure(new NotFoundError("No platform found for the given IDs."));
        }

        foreach (var platform in platformContainers)
        {
            var command = new DeleteContainerCommand
            (
                PlatformAddress: platform.Address,
                ContainerIds: platform.ContainersId,
                Volume: request.V,
                Force: request.Force,
                Link: request.Link
            );
            await connectorFactory.GetConnector(platform.ConnectorType).DeleteAsync(command, cancellationToken);
        }

        return Result.Success();
    }
}