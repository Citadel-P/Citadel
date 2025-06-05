using FluentValidation;
using Hosting.Common.ErrorTypes;
using Infrastructure.EntityFramework;
using Infrastructure.Services.Abstractions;
using Infrastructure.TaskJobs;
using LightResults;
using Mediator;
using Microsoft.EntityFrameworkCore;
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
    IPlatformHubDispatcher platformHubDispatcher,
    IPlatformHealthMonitorJob platformHealthMonitorJob,
    ILogger<DeletePlatformHandler> logger) : ICommandHandler<DeletePlatform, Result>
{
    public async ValueTask<Result> Handle(DeletePlatform command, CancellationToken cancellationToken)
    {
        var platform = await dbContext.Platforms
            .AsNoTracking()
            .SingleOrDefaultAsync(s => s.Id == command.Id, cancellationToken);

        if (platform is null)
        {
            return Result.Failure(new NotFoundError("Platform does not exists"));
        }

        dbContext.Platforms.Remove(platform);
        await dbContext.SaveChangesAsync(cancellationToken);

        await platformHealthMonitorJob.UntrackPlatform(platform.Address, cancellationToken);

        // Notify subscribers about the platform deletion
        await platformHubDispatcher.PlatformDeleted(command.Id);

        logger.LogInformation("Platform {Id} deleted successfully", command.Id);

        return Result.Success();
    }
}