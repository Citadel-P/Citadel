using Application.Services.Identity;
using Application.Services.Licensing;
using Application.Services;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Identity;
using Domain.Entities.Activities;
using Domain.Entities.Identity;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.Abstraction;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using Hosting.Common.MergePatch;
using LightResults;
using Mediator;

namespace Application.Features.Identity.ServiceAccounts;

[RequirePermission(ResourceType.ServiceAccount, PermissionLevel.Write, ResourceIdProperty = nameof(PatchServiceAccount.Id))]
public sealed record PatchServiceAccount(Guid Id, JsonMergePatchDocument<PatchServiceAccountModel> Patch)
    : ICommand<Result<ServiceAccountDetails>>, IAdministratorRequest
{
    internal sealed class Validator : PatchCommandValidator<PatchServiceAccount, PatchServiceAccountModel>
    {
        public Validator()
            : base(
                patchSelector: command => command.Patch,
                jsonTypeInfo: RoleJsonContext.Default.PatchServiceAccountModel,
                modelValidator: new PatchServiceAccountModelValidator())
        {
        }
    }

    private sealed class PatchServiceAccountModelValidator : AbstractValidator<PatchServiceAccountModel>
    {
        public PatchServiceAccountModelValidator()
            => RuleFor(model => model.Description).MaximumLength(600);
    }
}

internal sealed class PatchServiceAccountHandler(
    IUnitOfWork unitOfWork,
    IUserContextAccessor userContext,
    IActorScopeEvictor evictor,
    ILicenseEntitlementService entitlementService,
    TimeProvider timeProvider)
    : ICommandHandler<PatchServiceAccount, Result<ServiceAccountDetails>>
{
    public async ValueTask<Result<ServiceAccountDetails>> Handle(
        PatchServiceAccount command,
        CancellationToken cancellationToken)
    {
        var state = await ServiceAccountMutationState.LoadLockedAsync(unitOfWork, command.Id, cancellationToken);
        if (state is null)
            return Result.Failure<ServiceAccountDetails>(new NotFoundError("The Service Account does not exist."));

        var current = new PatchServiceAccountModel(state.Account.Description, state.Actor.IsEnabled);
        var patched = command.Patch.ApplyTo(current, RoleJsonContext.Default.PatchServiceAccountModel);
        if (patched.IsEnabled == true && !state.Actor.IsEnabled)
        {
            var entitlement = await entitlementService.EnsureEnabledAsync(
                LicenseCapability.CustomAccessControl,
                cancellationToken);
            if (entitlement.IsFailure(out var entitlementError))
                return Result.Failure<ServiceAccountDetails>(entitlementError);
        }

        var oldSnapshot = state.Snapshot();
        state.Account.Update(state.Account.Name, patched.Description, timeProvider.GetUtcNow().UtcDateTime);
        if (patched.IsEnabled.HasValue)
        {
            var enabled = state.Actor.SetEnabled(patched.IsEnabled.Value);
            if (enabled.IsFailure(out var enabledError))
                return Result.Failure<ServiceAccountDetails>(enabledError);
        }

        await unitOfWork.ServiceAccounts.UpdateAsync(state.Account, cancellationToken);
        await unitOfWork.Actors.UpdateAsync(state.Actor, cancellationToken);
        return await ServiceAccountMutationState.CompleteAsync(
            unitOfWork,
            userContext,
            evictor,
            state,
            oldSnapshot,
            cancellationToken);
    }
}

[RequirePermission(ResourceType.ServiceAccount, PermissionLevel.Write, ResourceIdProperty = nameof(RenameServiceAccount.Id))]
public sealed record RenameServiceAccount(Guid Id, string Name)
    : ICommand<Result<ServiceAccountDetails>>, IAdministratorRequest
{
    internal sealed class Validator : AbstractValidator<RenameServiceAccount>
    {
        public Validator()
        {
            RuleFor(command => command.Id).NotEmpty();
            RuleFor(command => command.Name).ValidNameIdentifier();
        }
    }
}

internal sealed class RenameServiceAccountHandler(
    IUnitOfWork unitOfWork,
    IUserContextAccessor userContext,
    IActorScopeEvictor evictor,
    TimeProvider timeProvider)
    : ICommandHandler<RenameServiceAccount, Result<ServiceAccountDetails>>
{
    public async ValueTask<Result<ServiceAccountDetails>> Handle(
        RenameServiceAccount command,
        CancellationToken cancellationToken)
    {
        var state = await ServiceAccountMutationState.LoadLockedAsync(unitOfWork, command.Id, cancellationToken);
        if (state is null)
            return Result.Failure<ServiceAccountDetails>(new NotFoundError("The Service Account does not exist."));

        await unitOfWork.ServiceAccounts.AcquireNameLockAsync(command.Name, cancellationToken);
        if (await unitOfWork.ServiceAccounts.ExistsByNameAsync(command.Name, command.Id, cancellationToken))
            return Result.Failure<ServiceAccountDetails>(new ConflictError("A Service Account with this name already exists."));

        var oldSnapshot = state.Snapshot();
        state.Account.Update(command.Name, state.Account.Description, timeProvider.GetUtcNow().UtcDateTime);
        await unitOfWork.ServiceAccounts.UpdateAsync(state.Account, cancellationToken);
        return await ServiceAccountMutationState.CompleteAsync(
            unitOfWork,
            userContext,
            evictor,
            state,
            oldSnapshot,
            cancellationToken);
    }
}

[RequirePermission(ResourceType.ServiceAccount, PermissionLevel.Write)]
public sealed record ArchiveServiceAccounts(IEnumerable<Guid> Ids) : ICommand<Result>, IAdministratorRequest
{
    internal sealed class Validator : AbstractValidator<ArchiveServiceAccounts>
    {
        public Validator()
            => RuleFor(command => command.Ids).NotNull().NotEmpty();
    }
}

internal sealed class ArchiveServiceAccountsHandler(
    IUnitOfWork unitOfWork,
    IUserContextAccessor userContext,
    IActorScopeEvictor evictor,
    TimeProvider timeProvider)
    : ICommandHandler<ArchiveServiceAccounts, Result>
{
    public async ValueTask<Result> Handle(ArchiveServiceAccounts command, CancellationToken cancellationToken)
    {
        var ids = command.Ids.Distinct().ToArray();
        var accounts = new List<ServiceAccount>(ids.Length);
        foreach (var id in ids)
        {
            var account = await unitOfWork.ServiceAccounts.GetAsync(id, cancellationToken);
            if (account is null)
                return Result.Failure(new NotFoundError("One or more Service Accounts do not exist."));
            accounts.Add(account);
        }

        foreach (var account in accounts.OrderBy(static account => account.ActorId))
            await unitOfWork.Actors.AcquireRunAsLockAsync(account.ActorId, cancellationToken);

        accounts.Clear();
        foreach (var id in ids)
        {
            var account = await unitOfWork.ServiceAccounts.GetAsync(id, cancellationToken);
            if (account is null)
                return Result.Failure(new NotFoundError("One or more Service Accounts do not exist."));
            accounts.Add(account);
        }

        var activeUsages = new List<RunAsActorUsage>();
        foreach (var account in accounts.Where(static account => !account.ArchivedAtUtc.HasValue))
        {
            activeUsages.AddRange((await unitOfWork.ServiceAccounts.GetRunAsUsagesAsync(
                account.ActorId,
                cancellationToken)).Where(static usage => usage.IsActive));
        }

        if (activeUsages.Count > 0)
        {
            var names = string.Join(", ", activeUsages.Select(static usage => usage.Name).Distinct().Take(3));
            var suffix = activeUsages.Select(static usage => usage.Name).Distinct().Skip(3).Any() ? ", …" : string.Empty;
            return Result.Failure(new ConflictError(
                $"The selected Service Accounts are used by {activeUsages.Count} active resource(s): {names}{suffix}. Replace those run-as bindings before archiving."));
        }

        var now = timeProvider.GetUtcNow().UtcDateTime;
        var affectedActorIds = new List<Guid>(accounts.Count);
        foreach (var account in accounts.Where(static account => !account.ArchivedAtUtc.HasValue))
        {
            var actor = await unitOfWork.Actors.GetById(account.ActorId, cancellationToken);
            if (actor is null)
                return Result.Failure(new NotFoundError("A Service Account Actor does not exist."));

            account.Archive(now);
            var disabled = actor.SetEnabled(false);
            if (disabled.IsFailure(out var disabledError))
                return Result.Failure(disabledError);
            await unitOfWork.ServiceAccounts.UpdateAsync(account, cancellationToken);
            await unitOfWork.Actors.UpdateAsync(actor, cancellationToken);
            await unitOfWork.ServiceAccounts.RevokeAllTokensAsync(
                account.Id,
                userContext.Current.ActorId,
                now,
                cancellationToken);
            await unitOfWork.ActivityEventRepository.AddAsync(
                ServiceAccountActivity.Create(
                    account,
                    userContext.Current.ActorId,
                    ActivityEventType.ServiceAccountArchived,
                    new ServiceAccountArchived(account.Id)),
                cancellationToken);
            affectedActorIds.Add(account.ActorId);
        }

        await unitOfWork.CommitAsync(cancellationToken);
        foreach (var actorId in affectedActorIds)
            await evictor.EvictActorAsync(actorId, CancellationToken.None);
        return Result.Success();
    }
}

internal sealed record ServiceAccountMutationState(
    ServiceAccount Account,
    Actor Actor,
    Guid[] TeamIds,
    Guid[] RoleIds,
    ResourceAccess[] ResourceAccesses)
{
    internal ServiceAccountActivitySnapshot Snapshot()
        => ServiceAccountActivity.Snapshot(Account, Actor.IsEnabled, TeamIds, RoleIds, ResourceAccesses);

    internal static async Task<ServiceAccountMutationState?> LoadAsync(
        IUnitOfWork unitOfWork,
        Guid id,
        CancellationToken cancellationToken)
    {
        var account = await unitOfWork.ServiceAccounts.GetAsync(id, cancellationToken);
        if (account is null || account.ArchivedAtUtc.HasValue)
            return null;
        var actor = await unitOfWork.Actors.GetById(account.ActorId, cancellationToken);
        if (actor is null)
            return null;
        var teamIds = await unitOfWork.ServiceAccounts.GetTeamIdsAsync(actor.Id, cancellationToken);
        var roleIds = (await unitOfWork.Roles.GetActorRoleIdsAsync(actor.Id, cancellationToken)).ToArray();
        var resourceAccesses = (await unitOfWork.ResourceAccesses.GetAllByActorIdAsync(actor.Id, cancellationToken))
            .Select(static access => ResourceAccess.FromPersistence(
                access.Id,
                access.ResourceType,
                access.ResourceId,
                access.ActorId,
                access.PermissionLevel,
                access.SpecificPermissions))
            .ToArray();
        return new(account, actor, teamIds.ToArray(), roleIds, resourceAccesses);
    }

    internal static async Task<ServiceAccountMutationState?> LoadLockedAsync(
        IUnitOfWork unitOfWork,
        Guid id,
        CancellationToken cancellationToken)
    {
        var account = await unitOfWork.ServiceAccounts.GetAsync(id, cancellationToken);
        if (account is null)
            return null;
        await unitOfWork.Actors.AcquireRunAsLockAsync(account.ActorId, cancellationToken);
        return await LoadAsync(unitOfWork, id, cancellationToken);
    }

    internal static async Task<Result<ServiceAccountDetails>> CompleteAsync(
        IUnitOfWork unitOfWork,
        IUserContextAccessor userContext,
        IActorScopeEvictor evictor,
        ServiceAccountMutationState state,
        ServiceAccountActivitySnapshot oldSnapshot,
        CancellationToken cancellationToken,
        Guid[]? teamIds = null,
        Guid[]? roleIds = null,
        ResourceAccess[]? resourceAccesses = null)
    {
        var newSnapshot = ServiceAccountActivity.Snapshot(
            state.Account,
            state.Actor.IsEnabled,
            teamIds ?? state.TeamIds,
            roleIds ?? state.RoleIds,
            resourceAccesses ?? state.ResourceAccesses);
        var (eventType, info) = ServiceAccountActivity.DescribeUpdate(
            state.Account.Id,
            oldSnapshot.Name,
            state.Account.Name,
            oldSnapshot.IsEnabled,
            state.Actor.IsEnabled,
            oldSnapshot,
            newSnapshot);
        await unitOfWork.ActivityEventRepository.AddAsync(
            ServiceAccountActivity.Create(state.Account, userContext.Current.ActorId, eventType, info),
            cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);
        await evictor.EvictActorAsync(state.Actor.Id, CancellationToken.None);
        return Result.Success((await GetServiceAccountHandler.LoadDetailsAsync(
            unitOfWork,
            state.Account.Id,
            cancellationToken))!);
    }
}
