using Application.Services.SignalR;
using Application.TaskJobs;
using Domain.Contracts.Interfaces;
using FluentValidation;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;
using Microsoft.Extensions.Logging;

namespace Application.Features.Platforms.Commands;

public sealed record DeletePlatforms(IEnumerable<Guid> Ids) : ICommand<Result>
{
    internal sealed class Validator : AbstractValidator<DeletePlatforms>
    {
        public Validator()
            => RuleForEach(s => s.Ids).NotNull();
    }
}

internal class DeletePlatformHandler(
    IUnitOfWork unitOfWork,
    IPlatformStreamManager platformStreamManager,
    IPlatformHealthMonitorJob platformHealthMonitorJob,
    ILogger<DeletePlatformHandler> logger) : ICommandHandler<DeletePlatforms, Result>
{
    public async ValueTask<Result> Handle(DeletePlatforms command, CancellationToken cancellationToken)
    {
        foreach (var platformId in command.Ids)
        {
            var platform = await unitOfWork.Platforms.GetPlatformInfoAsync(platformId, cancellationToken);
            if (platform is null)
            {
                return Result.Failure(new NotFoundError("Platform does not exist"));
            }

            await unitOfWork.Platforms.DeleteAsync(platformId, cancellationToken);
            await unitOfWork.CommitAsync();

            await platformHealthMonitorJob.UntrackPlatform(platform.Address, cancellationToken);

            // Notify subscribers
            await platformStreamManager.PlatformDeleted(platformId);

            logger.LogInformation("Platform {Id} deleted successfully", platformId);
        }
        
        return Result.Success();
    }
}