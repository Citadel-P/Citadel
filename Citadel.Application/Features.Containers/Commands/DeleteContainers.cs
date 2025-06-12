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

public sealed record DeleteContainers(string[] ContainersIds, bool? V = false, bool? Force = false, bool? Link = false) : ICommand<Result>
{
    internal class Validator : AbstractValidator<DeleteContainers>
    {
        public Validator()
            => RuleForEach(s => s.ContainersIds).ValidContainerId();
    }
}

internal sealed class DeleteContainersHandler(IContainerService containerService, ApplicationDbContext dbContext) : ICommandHandler<DeleteContainers, Result>
{
    public async ValueTask<Result> Handle(DeleteContainers request, CancellationToken cancellationToken)
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

        var command = new DeleteContainerCommand
        (
            PlatformContainers: platformContainers,
            Verbose: request.V,
            Force: request.Force,
            Link: request.Link
        );
        return await containerService.DeleteAsync(command, cancellationToken);
    }
}