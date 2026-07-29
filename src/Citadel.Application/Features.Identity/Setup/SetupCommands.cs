using Application.Features.Identity.Auth.Models;
using Application.Services.Identity;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Identity;
using Domain.Entities.Activities;
using Domain.Entities.Identity;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.Identity.Setup;

public sealed record SetupStatus(bool RequiresSetup);

public sealed record GetSetupStatus : IQuery<Result<SetupStatus>>;

public sealed record InitializeCitadel(
    string Name,
    string Email,
    string Password) : ICommand<Result<LoginResponse>>
{
    internal sealed class Validator : AbstractValidator<InitializeCitadel>
    {
        public Validator()
        {
            RuleFor(x => x.Name).ValidNameIdentifier();
            RuleFor(x => x.Email).NotEmpty().EmailAddress();
            RuleFor(x => x).Custom((command, context) =>
            {
                var error = LocalPasswordPolicy.GetValidationError(
                    command.Password,
                    command.Name,
                    command.Email);
                if (error is not null)
                    context.AddFailure(nameof(command.Password), error);
            });
        }
    }
}

internal sealed record InitializeCitadelUnattended(
    string Name,
    string Email,
    string Password) : ICommand<Result>
{
    internal sealed class Validator : AbstractValidator<InitializeCitadelUnattended>
    {
        public Validator()
        {
            RuleFor(x => x.Name).ValidNameIdentifier();
            RuleFor(x => x.Email).NotEmpty().EmailAddress();
            RuleFor(x => x).Custom((command, context) =>
            {
                var error = LocalPasswordPolicy.GetValidationError(
                    command.Password,
                    command.Name,
                    command.Email);
                if (error is not null)
                    context.AddFailure(nameof(command.Password), error);
            });
        }
    }
}

internal sealed class GetSetupStatusHandler(
    IUnitOfWork unitOfWork,
    ISetupStateCache cache)
    : IQueryHandler<GetSetupStatus, Result<SetupStatus>>
{
    public async ValueTask<Result<SetupStatus>> Handle(
        GetSetupStatus query,
        CancellationToken cancellationToken)
    {
        if (cache.TryGetRequiresSetup(out var cached))
            return Result.Success(new SetupStatus(cached));

        var state = await unitOfWork.InstanceSetupState.GetAsync(cancellationToken);
        if (state is null)
        {
            return Result.Failure<SetupStatus>(
                new ServiceUnavailableError("Citadel setup state is unavailable."));
        }

        cache.SetRequiresSetup(state.RequiresSetup);
        return Result.Success(new SetupStatus(state.RequiresSetup));
    }
}

internal sealed class InitializeCitadelHandler(
    InitialAdministratorService initializer,
    ILocalAuthenticationCompletionService authenticationCompletion)
    : ICommandHandler<InitializeCitadel, Result<LoginResponse>>
{
    public async ValueTask<Result<LoginResponse>> Handle(
        InitializeCitadel command,
        CancellationToken cancellationToken)
    {
        var result = await initializer.InitializeAsync(
            command.Name,
            command.Email,
            command.Password,
            SetupInitializationMode.Interactive,
            cancellationToken);
        if (!result.IsSuccess(out var user))
            return Result.Failure<LoginResponse>(result.Errors);

        return Result.Success(
            await authenticationCompletion.CompleteAsync(user, cancellationToken));
    }
}

internal sealed class InitializeCitadelUnattendedHandler(
    InitialAdministratorService initializer)
    : ICommandHandler<InitializeCitadelUnattended, Result>
{
    public async ValueTask<Result> Handle(
        InitializeCitadelUnattended command,
        CancellationToken cancellationToken)
    {
        var result = await initializer.InitializeAsync(
            command.Name,
            command.Email,
            command.Password,
            SetupInitializationMode.Unattended,
            cancellationToken);
        return result.IsSuccess(out _)
            ? Result.Success()
            : Result.Failure(result.Errors);
    }
}

internal enum SetupInitializationMode
{
    Interactive,
    Unattended
}

internal sealed class InitialAdministratorService(
    IUnitOfWork unitOfWork,
    ICitadelPasswordHasher passwordHasher,
    ISetupStateCache cache)
{
    private static readonly Guid AdminRoleId =
        Guid.Parse("30000000-0000-0000-0000-000000000001");

    public async Task<Result<UserAuthInfo>> InitializeAsync(
        string name,
        string email,
        string password,
        SetupInitializationMode mode,
        CancellationToken cancellationToken)
    {
        var state = await unitOfWork.InstanceSetupState.GetLockedAsync(cancellationToken);
        if (state is null)
        {
            return Result.Failure<UserAuthInfo>(
                new ServiceUnavailableError("Citadel setup state is unavailable."));
        }

        if (!state.RequiresSetup)
        {
            cache.SetRequiresSetup(false);
            return Result.Failure<UserAuthInfo>(
                new ConflictError(
                    "Citadel setup is already complete.",
                    new Dictionary<string, object>
                    {
                        ["problemType"] = "setup_already_complete"
                    }));
        }

        var conflicts = await unitOfWork.Users.GetConflictsAsync(
            name,
            email,
            null,
            cancellationToken);
        if (conflicts.NameExists)
            return Result.Failure<UserAuthInfo>(new ConflictError("Name already exists."));
        if (conflicts.EmailExists)
            return Result.Failure<UserAuthInfo>(new ConflictError("Email already exists."));

        var actor = Actor.Create(ActorType.User, new ActorMetadata(name));
        var user = new User(
            name,
            email,
            passwordHasher.Hash(password),
            actor.Id,
            Constants.SystemId);

        if (!state.TryComplete(actor.Id, DateTimeOffset.UtcNow))
        {
            return Result.Failure<UserAuthInfo>(
                new ConflictError("Citadel setup is already complete."));
        }

        await unitOfWork.Actors.AddAsync(actor, cancellationToken);
        await unitOfWork.Users.AddAsync(user, cancellationToken);
        await unitOfWork.Roles.AddActorRoleAsync(actor.Id, AdminRoleId, cancellationToken);

        foreach (var defaultAction in DefaultAutomationActions.Create(actor.Id))
        {
            var affectedRows = await unitOfWork.AutomationActions.AddAsync(
                defaultAction.Action,
                cancellationToken,
                defaultAction.TagIds,
                Constants.SystemId);
            if (affectedRows == 0)
            {
                throw new InvalidOperationException(
                    "Citadel default automation actions could not be created because a seeded tag is missing.");
            }
        }

        await unitOfWork.InstanceSetupState.UpdateAsync(state, cancellationToken);
        await unitOfWork.ActivityEventRepository.AddAsync(
            new ActivityEvent(
                platformId: null,
                resourceId: user.Id,
                actorId: Constants.SystemId,
                resourceName: user.Name,
                eventType: ActivityEventType.InitialAdministratorCreated,
                status: ActivityStatus.Success,
                info: new InitialAdministratorCreated(user.Id, user.Name, mode.ToString())),
            cancellationToken);

        await unitOfWork.CommitAsync(cancellationToken);
        cache.SetRequiresSetup(false);
        return Result.Success(
            new UserAuthInfo(
                user.Id,
                actor.Id,
                user.Name,
                user.Email,
                user.Password,
                ["Admin"]));
    }
}
