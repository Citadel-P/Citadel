using Application.Services;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Identity;
using Domain.Entities.Activities;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.Abstraction;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.Identity.Profile.Commands;

public sealed record UpdateCurrentProfile(string DisplayName) : ICommand<Result<CurrentProfileDetails>>
{
    internal sealed class Validator : AbstractValidator<UpdateCurrentProfile>
    {
        public Validator()
        {
            RuleFor(x => x.DisplayName).ValidNameIdentifier();
        }
    }
}

internal sealed class UpdateCurrentProfileHandler(IUnitOfWork unitOfWork, IUserContextAccessor userContext)
    : ICommandHandler<UpdateCurrentProfile, Result<CurrentProfileDetails>>
{
    public async ValueTask<Result<CurrentProfileDetails>> Handle(UpdateCurrentProfile command, CancellationToken cancellationToken)
    {
        var userId = userContext.Current.UserId;
        var actorId = userContext.Current.ActorId;
        if (userId == Guid.Empty || actorId == Guid.Empty)
            return Result.Failure<CurrentProfileDetails>(new UnauthorizedError("Missing user context"));

        var user = await unitOfWork.Users.GetAsync(userId, cancellationToken);
        if (user is null)
            return Result.Failure<CurrentProfileDetails>(new NotFoundError("Current user does not exist"));

        var displayName = command.DisplayName.Trim();
        if (!string.Equals(user.Name, displayName, StringComparison.Ordinal))
        {
            if (await unitOfWork.Users.ExistsByNameAsync(displayName, user.Id, cancellationToken))
                return Result.Failure<CurrentProfileDetails>(new ConflictError("Name already exists"));

            var oldName = user.Name;
            user.UpdateMetadata(name: displayName);
            await unitOfWork.Users.UpdateAsync(user, cancellationToken);
            await unitOfWork.ActivityEventRepository.AddAsync(
                new ActivityEvent(
                    platformId: null,
                    resourceId: user.Id,
                    actorId: actorId,
                    resourceName: displayName,
                    eventType: ActivityEventType.UserProfileUpdated,
                    status: ActivityStatus.Success,
                    info: new UserProfileUpdated([
                        new ActivityChangedField("DisplayName", oldName, displayName)
                    ])),
                cancellationToken);

            await unitOfWork.CommitAsync(cancellationToken);
        }

        var profile = await unitOfWork.Users.GetCurrentProfileAsync(userId, cancellationToken);
        return profile is null
            ? Result.Failure<CurrentProfileDetails>(new NotFoundError("Current user does not exist"))
            : Result.Success(profile);
    }
}
