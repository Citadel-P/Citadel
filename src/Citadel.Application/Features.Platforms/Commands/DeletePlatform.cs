using Application.Services.Abstractions;
using Application.Services.SignalR;
using Application.TaskJobs;
using Domain.Contracts.Interfaces;
using FluentValidation;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;
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
    IUnitOfWork unitOfWork,
    IPlatformsStreamManager platformStreamManager,
    IPlatformHealthMonitorJob platformHealthMonitorJob,
    ILogger<DeletePlatformHandler> logger) : ICommandHandler<DeletePlatform, Result>
{
    public async ValueTask<Result> Handle(DeletePlatform command, CancellationToken cancellationToken)
    {
        var platform = await unitOfWork.Platforms.GetPlatformInfoAsync(command.Id, cancellationToken);
        if (platform is null)
        {
            return Result.Failure(new NotFoundError("Platform does not exist"));
        }

        await unitOfWork.Platforms.DeleteAsync(command.Id, cancellationToken);
        await unitOfWork.CommitAsync();

        await platformHealthMonitorJob.UntrackPlatform(platform.Address, cancellationToken);

        // Notify subscribers
        await platformStreamManager.PlatformDeleted(command.Id);

        logger.LogInformation("Platform {Id} deleted successfully", command.Id);

        return Result.Success();
    }
}