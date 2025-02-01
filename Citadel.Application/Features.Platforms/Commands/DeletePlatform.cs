using Hosting.Common.ErrorTypes;
using Infrastructure.Entities;
using FluentValidation;
using LightResults;
using Mediator;
using Microsoft.EntityFrameworkCore;
using Infrastructure.EntityFramework;
using Infrastructure.Services;
using Quartz;
using Infrastructure.TaskJobs;
using Microsoft.Extensions.Logging;

namespace Application.Features.Platforms.Commands;

public sealed record DeletePlatform(Guid Id) : ICommand<Result>
{
    internal sealed class Validator : AbstractValidator<DeletePlatform>
    {
        public Validator()
            => RuleFor(s => s.Id).NotNull();
    }
}

internal class DeletePlatformHandler(
    ApplicationDbContext dbContext, 
    ISchedulerFactory schedulerFactory,
    ICacheService cacheService,
    ILogger<DeletePlatformHandler> logger) : ICommandHandler<DeletePlatform, Result>
{
    public async ValueTask<Result> Handle(DeletePlatform command, CancellationToken cancellationToken)
    {
        Platform platform = await dbContext.Platforms
            .AsNoTracking()
            .Include(s => s.SystemInfo)
            .SingleOrDefaultAsync(s => s.Id == command.Id, cancellationToken);

        if (platform is null)
        {
            return Result.Failure(new NotFoundError("Platform does not exists"));
        }

        dbContext.Platforms.Remove(platform);
        await dbContext.SaveChangesAsync(cancellationToken);

        cacheService.DeletePlatformId(platform.SystemInfo.DaemonId);
        await AbortTaskJobs(platform.Address, cancellationToken);

        return Result.Success();
    }

    private async Task AbortTaskJobs(string address, CancellationToken cancellationToken)
    {
        var scheduler = await schedulerFactory.GetScheduler(cancellationToken);
        await scheduler.AbortStreamDaemonEventJob(address, logger, cancellationToken);
    }
}