using FluentValidation;
using Hosting.Common.ErrorTypes;
using Infrastructure.Entities;
using Infrastructure.EntityFramework;
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
    IDaemonEventJob daemonEventJob,
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

        daemonEventJob.StopMonitoringPlatform(platform.Address);

        return Result.Success();
    }
}