using Application.Services.SignalR;
using Application.TaskJobs;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Platforms;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using Hosting.Common.MergePatch;
using LightResults;
using Mediator;

namespace Application.Features.Platforms.Commands;

[RequirePermission(ResourceType.Platform, PermissionLevel.Write)]
public sealed record PatchPlatformMetadata(Guid Id, JsonMergePatchDocument<Platform> Patch) : ICommand<Result<Platform>>
{
    internal sealed class Validator : PatchCommandValidator<PatchPlatformMetadata, Platform>
    {
        public Validator()
            : base(
                  patchSelector: x => x.Patch,
                  jsonTypeInfo: PlatformJsonContext.Default.Platform,
                  modelValidator: new PlatformValidator())
        {
        }
    }

    internal sealed class PlatformValidator : AbstractValidator<Platform>
    {
        public PlatformValidator()
        {
            RuleFor(x => x.Id).NotEmpty().NotNull();
            When(x => x.Description is not null, () => RuleFor(x => x.Description).MaximumLength(600));
        }
    }
}

internal sealed class PatchPlatformMetadataHandler(
    IUnitOfWork unitOfWork,
    IPlatformStreamManager platformHub,
    INotificationQueue notificationQueue) : ICommandHandler<PatchPlatformMetadata, Result<Platform>>
{
    public async ValueTask<Result<Platform>> Handle(PatchPlatformMetadata command, CancellationToken cancellationToken)
    {
        var platform = await unitOfWork.Platforms.GetByIdAsync(command.Id, cancellationToken);
        if (platform == null)
        {
            return Result.Failure<Platform>(new NotFoundError("The provided platform does not exist"));
        }

        var patchedPlatform = command.Patch.ApplyTo(platform, PlatformJsonContext.Default.Platform);

        platform.PartialUpdate(description: patchedPlatform.Description);

        await unitOfWork.Platforms.UpdateAsync(platform, cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);

        await notificationQueue.EnqueueAsync(new PushPlatformUpdateNotificationWorkItem(platformHub, platform), cancellationToken);

        return platform;
    }
}
