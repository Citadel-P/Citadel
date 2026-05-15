using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Identity;
using Domain.Entities.Identity;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using Hosting.Common.Extensions;
using LightResults;
using Mediator;
using Microsoft.AspNetCore.Http;
using Application.Services.Identity;

namespace Application.Features.Identity.Users.Commands;

[RequirePermission(ResourceType.User, PermissionLevel.Write)]
public sealed record CreateUser(
    string Name,
    string Email,
    string Password,
    bool IsEnabled = true,
    IEnumerable<Guid>? TeamIds = null,
    IEnumerable<Guid>? RoleIds = null,
    IEnumerable<ResourceAccessView>? ResourceAccesses = null) : ICommand<Result<UserDetails>>
{
    internal sealed class Validator : AbstractValidator<CreateUser>
    {
        public Validator()
        {
            RuleFor(x => x.Name).ValidNameIdentifier();
            RuleFor(x => x.Email).NotEmpty().EmailAddress();
            RuleFor(x => x.Password).NotNull().MinimumLength(6).MaximumLength(128);

            RuleForEach(x => x.TeamIds).NotEmpty();
            RuleForEach(x => x.RoleIds).NotEmpty();

            RuleForEach(x => x.ResourceAccesses)
                .Must(x => PermissionMatrix.IsAllowed(x.ResourceType, x.PermissionLevel, x.SpecificPermissions))
                .WithMessage("Invalid permission combination in resource accesses.");
        }
    }
}

internal sealed class CreateUserHandler(IUnitOfWork unitOfWork, IHttpContextAccessor httpContextAccessor, IActorScopeEvictor evictor) : ICommandHandler<CreateUser, Result<UserDetails>>
{
    public async ValueTask<Result<UserDetails>> Handle(CreateUser command, CancellationToken cancellationToken)
    {
        var actorId = httpContextAccessor.HttpContext?.User?.GetActorId()
            ?? throw new ArgumentNullException("ActorId claim is missing");

        var conflicts = await unitOfWork.Users.GetConflictsAsync(command.Name, command.Email, null, cancellationToken);
        if (conflicts.NameExists)
            return Result.Failure<UserDetails>(new ConflictError("Name already exists"));

        if (conflicts.EmailExists)
            return Result.Failure<UserDetails>(new ConflictError("Email already exists"));

        var userActor = Actor.Create(ActorType.User, new ActorMetadata(command.Name), command.IsEnabled);
        var user = new User(command.Name, command.Email, command.Password, userActor.Id, actorId);

        await unitOfWork.Actors.AddAsync(userActor, cancellationToken);
        await unitOfWork.Users.AddAsync(user, cancellationToken);

        var teamIds = command.TeamIds?.Distinct().ToArray() ?? [];
        var roleIds = command.RoleIds?.Distinct().ToArray() ?? [];
        var resourceAccesses = command.ResourceAccesses?.Distinct().ToArray() ?? [];

        if (teamIds.Length > 0)
        {
            var teams = await unitOfWork.Teams.GetAllAsync(teamIds, cancellationToken) ?? [];
            var existingTeamIds = teams.Select(x => x.Id).ToHashSet();
            var missingTeamId = teamIds.FirstOrDefault(x => !existingTeamIds.Contains(x));
            if (missingTeamId != Guid.Empty)
                return Result.Failure<UserDetails>(new NotFoundError($"Team with ID {missingTeamId} does not exist"));

            await unitOfWork.Users.ReplaceTeamsAsync(user.Id, teamIds, cancellationToken);
            evictor.EvictUsersAsync([user.Id], cancellationToken);
        }

        if (roleIds.Length > 0)
        {
            var roles = await unitOfWork.Roles.GetAllAsync(roleIds, cancellationToken) ?? [];
            var existingRoleIds = roles.Select(x => x.Id).ToHashSet();
            var missingRoleId = roleIds.FirstOrDefault(x => !existingRoleIds.Contains(x));
            if (missingRoleId != Guid.Empty)
                return Result.Failure<UserDetails>(new NotFoundError($"Role with ID {missingRoleId} does not exist"));

            await unitOfWork.Roles.ReplaceActorRolesAsync(user.ActorId, roleIds, cancellationToken);
            await evictor.EvictForActorAsync(user.ActorId, cancellationToken);
        }

        if (resourceAccesses.Length > 0)
        {
            var accessRows = resourceAccesses
                .Select(x => ResourceAccess.Create(x.ResourceType, x.ResourceId, user.ActorId, x.PermissionLevel, x.SpecificPermissions))
                .ToArray();

            await unitOfWork.ResourceAccesses.ReplaceAsync(user.ActorId, accessRows, cancellationToken);
        }

        await unitOfWork.CommitAsync(cancellationToken);

        var persistedResourceAccesses = await unitOfWork.ResourceAccesses.GetAllByActorIdAsync(user.ActorId, cancellationToken);
        return new UserDetails(
            user.Id,
            user.Name,
            user.Email,
            user.ActorId,
            userActor.IsEnabled,
            user.CreatedAt,
            user.CreatedByActorId,
            ResourceAccesses: persistedResourceAccesses.Select(x => new ResourceAccessView(x.ResourceType, x.ResourceId, x.ResourceName, x.PermissionLevel, x.SpecificPermissions)));
    }
}
