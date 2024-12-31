using Application.Features.Containers.Models;
using Application.Services.Abstractions;
using Application.Utils;
using Hosting.Common.ErrorTypes;
using Infrastructure.EntityFramework;
using Infrastructure.Services;
using LightResults;
using Mediator;
using Microsoft.EntityFrameworkCore;

namespace Application.Features.Containers.Commands;

public sealed record OnContainerEvent(ContainerEventRequest Input) : ICommand<Result>;

internal class OnContainerEventHandler
    (ICacheService cacheService,
    ApplicationDbContext dbContext,
    IContainerHubDispatcher containerHub) : ICommandHandler<OnContainerEvent, Result>
{
    public async ValueTask<Result> Handle(OnContainerEvent command, CancellationToken cancellationToken)
    {
        Guid? platformId = await cacheService.GetPlatformId(command.Input.Id, cancellationToken);
        if (platformId == null)
        {
            return Result.Failure(new NotFoundError($"Platform does not exists, daemon id: {command.Input.Id}"));
        }

        var containerInfo = command.Input.ContainerInfo.Map(platformId.Value);

        if (command.Input.Action == "create")
        {
            dbContext.ContainersInfo.Add(containerInfo);
            await dbContext.SaveChangesAsync(cancellationToken);
        }
        else if (command.Input.Action == "destroy")
        {
            var existing = await dbContext.ContainersInfo.FirstOrDefaultAsync(s => s.ContainerId == command.Input.ContainerId, cancellationToken);
            dbContext.ContainersInfo.Remove(existing);
            await dbContext.SaveChangesAsync(cancellationToken);
        }
        else
        {
            var existing = await dbContext.ContainersInfo.FirstOrDefaultAsync(s => s.ContainerId == command.Input.ContainerId, cancellationToken);
            if (existing != null)
            {
                var state = (command.Input.Action) switch
                {
                    "stop" => "exited",
                    "start" => "running",
                    "pause" => "paused",
                    _ => throw new NotImplementedException()
                };
                existing.UpdateWith(state: state, status: containerInfo.Status);

                await dbContext.SaveChangesAsync(cancellationToken);
            }
        }

        var containers = await dbContext.ContainersInfo.WithLastStat(platformId.Value).ToListAsync(cancellationToken);
        await containerHub.SendContainersInfo(containers.OrderByDescending(s => s.Created));

        return Result.Success();
    }
}
