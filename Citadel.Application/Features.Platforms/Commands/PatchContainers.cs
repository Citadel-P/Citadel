using FluentValidation;
using LightResults;
using Mediator;
using Microsoft.EntityFrameworkCore;
using Hosting.Common;
using Infrastructure.EntityFramework;
using Infrastructure.Services;

namespace Application.Features.Platforms.Commands;

public sealed record PatchContainers(string[] ContainersIds, ContainerAction Action) : ICommand<Result>
{
    internal class Validator : AbstractValidator<PatchContainers>
    {
        public Validator()
            => RuleForEach(s => s.ContainersIds).ValidContainerId();
    }
}

public enum ContainerAction : uint
{
    START = 0,
    RESTART,
    STOP,
    PAUSE,
    UNPAUSE,
    DELETE
}

internal class PatchContainersHandler(
    IAgentService agentService,
    ApplicationDbContext dbContext)
    : ICommandHandler<PatchContainers, Result>
{
    public async ValueTask<Result> Handle(PatchContainers request, CancellationToken cancellationToken)
    {
        var containers = await dbContext.ContainersInfo.Include(s => s.Platform)
                        .AsNoTracking()
                        .Where(s => request.ContainersIds.Contains(s.ContainerId))
                        .ToListAsync(cancellationToken);

        foreach (var container in containers.GroupBy(s => s.Platform.Address)) 
        {
            var containerIds = new string[container.Count()];
            for (var i = 0; i < containerIds.Length; i++)
                containerIds[i] = container.ElementAt(i).ContainerId;
            
            var response = await ToOperation(container.Key, containerIds, request.Action);
            if (response.IsFailure(out var error))
            {
                return Result.Failure(error);
            }
        }

        async Task<Result> ToOperation(string platformAddress, string[] containersIds, ContainerAction action) => action switch
        {
            ContainerAction.START => await agentService.StartContainers(platformAddress, containersIds, cancellationToken),
            ContainerAction.STOP => await agentService.StopContainers(platformAddress, containersIds, cancellationToken),
            ContainerAction.PAUSE => await agentService.PauseContainers(platformAddress, containersIds, cancellationToken),
            ContainerAction.UNPAUSE => await agentService.UnpauseContainers(platformAddress, containersIds, cancellationToken),
            ContainerAction.DELETE => await agentService.DeleteContainers(platformAddress, containersIds, cancellationToken),
            ContainerAction.RESTART => await agentService.RestartContainers(platformAddress, containersIds, cancellationToken),
            _ => throw new NotImplementedException()
        };


        return Result.Success();
    }
}