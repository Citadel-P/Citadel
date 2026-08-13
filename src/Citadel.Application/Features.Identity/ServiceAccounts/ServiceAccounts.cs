using System.Security.Cryptography;
using Application.Services.Identity;
using Application.Services.Licensing;
using Domain;
using Domain.Configs;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Identity;
using Domain.Entities.Activities;
using Domain.Entities.Identity;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.Attributes;
using Hosting.Common.Abstraction;
using Hosting.Common.ErrorTypes;
using Hosting.Common.Models;
using LightResults;
using Mediator;
using Microsoft.Extensions.Options;

namespace Application.Features.Identity.ServiceAccounts;

[RequirePermission(ResourceType.ServiceAccount, PermissionLevel.Read)]
public sealed record GetServiceAccounts(
    int Page = 1,
    int PageSize = 50,
    string? Name = null,
    bool IncludeArchived = false) : IQuery<Result<PagedResult<ServiceAccountDetails>>>
{
    internal sealed class Validator : AbstractValidator<GetServiceAccounts>
    {
        public Validator()
        {
            RuleFor(x => x.Page).GreaterThan(0);
            RuleFor(x => x.PageSize).InclusiveBetween(1, 100);
            RuleFor(x => x.Name).MaximumLength(128);
        }
    }
}

internal sealed class GetServiceAccountsHandler(IUnitOfWork unitOfWork, IUserContextAccessor userContext)
    : IQueryHandler<GetServiceAccounts, Result<PagedResult<ServiceAccountDetails>>>
{
    public async ValueTask<Result<PagedResult<ServiceAccountDetails>>> Handle(
        GetServiceAccounts query,
        CancellationToken cancellationToken)
    {
        var principal = userContext.Current;
        var result = principal.IsAdmin
            ? await unitOfWork.ServiceAccounts.GetPagedAsync(
                query.Page,
                query.PageSize,
                query.Name,
                query.IncludeArchived,
                cancellationToken)
            : await unitOfWork.ServiceAccounts.GetAuthorizedPagedAsync(
                principal.ActorId,
                PermissionLevel.Read,
                SpecificPermission.None,
                query.Page,
                query.PageSize,
                query.Name,
                query.IncludeArchived,
                cancellationToken);
        return Result.Success(result);
    }
}

[RequirePermission(ResourceType.ServiceAccount, PermissionLevel.Read, ResourceIdProperty = nameof(GetServiceAccount.Id))]
public sealed record GetServiceAccount(Guid Id) : IQuery<Result<ServiceAccountDetails>>;

internal sealed class GetServiceAccountHandler(IUnitOfWork unitOfWork)
    : IQueryHandler<GetServiceAccount, Result<ServiceAccountDetails>>
{
    public async ValueTask<Result<ServiceAccountDetails>> Handle(GetServiceAccount query, CancellationToken cancellationToken)
    {
        var account = await LoadDetailsAsync(unitOfWork, query.Id, cancellationToken);
        return account is null
            ? Result.Failure<ServiceAccountDetails>(new NotFoundError("The Service Account does not exist."))
            : Result.Success(account);
    }

    internal static async Task<ServiceAccountDetails?> LoadDetailsAsync(
        IUnitOfWork unitOfWork,
        Guid id,
        CancellationToken cancellationToken)
    {
        var account = await unitOfWork.ServiceAccounts.GetDetailsAsync(id, cancellationToken);
        if (account is null)
            return null;
        var accesses = await unitOfWork.ResourceAccesses.GetAllByActorIdAsync(account.ActorId, cancellationToken);
        return account with
        {
            ResourceAccesses = accesses.Select(x => new ResourceAccessView(
                x.ResourceType,
                x.ResourceId,
                x.ResourceName,
                x.PermissionLevel,
                x.SpecificPermissions,
                x.Id)),
        };
    }
}

[RequirePermission(ResourceType.ServiceAccount, PermissionLevel.Write)]
public sealed record CreateServiceAccount(
    string Name,
    string? Description,
    bool IsEnabled = true,
    IEnumerable<Guid>? TeamIds = null,
    IEnumerable<Guid>? RoleIds = null,
    IEnumerable<ResourceAccessView>? ResourceAccesses = null)
    : ICommand<Result<ServiceAccountDetails>>, IAdministratorRequest
{
    internal sealed class Validator : AbstractValidator<CreateServiceAccount>
    {
        public Validator()
        {
            RuleFor(x => x.Name).ValidNameIdentifier();
            RuleFor(x => x.Description).MaximumLength(600);
            RuleForEach(x => x.TeamIds).NotEmpty();
            RuleForEach(x => x.RoleIds).NotEmpty();
            RuleForEach(x => x.ResourceAccesses)
                .Must(x => PermissionMatrix.IsAllowed(x.ResourceType, x.PermissionLevel, x.SpecificPermissions))
                .WithMessage("Invalid permission combination in resource accesses.");
        }
    }
}

internal sealed class CreateServiceAccountHandler(
    IUnitOfWork unitOfWork,
    IUserContextAccessor userContext,
    IActorScopeEvictor evictor,
    ILicenseEntitlementService entitlementService,
    TimeProvider timeProvider)
    : ICommandHandler<CreateServiceAccount, Result<ServiceAccountDetails>>
{
    public async ValueTask<Result<ServiceAccountDetails>> Handle(CreateServiceAccount command, CancellationToken cancellationToken)
    {
        var entitlement = await entitlementService.EnsureEnabledAsync(LicenseCapability.CustomAccessControl, cancellationToken);
        if (entitlement.IsFailure(out var entitlementError))
            return Result.Failure<ServiceAccountDetails>(entitlementError);

        await unitOfWork.ServiceAccounts.AcquireNameLockAsync(command.Name, cancellationToken);
        if (await unitOfWork.ServiceAccounts.ExistsByNameAsync(command.Name, null, cancellationToken))
            return Result.Failure<ServiceAccountDetails>(new ConflictError("A Service Account with this name already exists."));

        var teamsResult = await ValidateTeamsAsync(unitOfWork, entitlementService, command.TeamIds, cancellationToken);
        if (!teamsResult.IsSuccess(out var teamIds, out var teamsError))
            return Result.Failure<ServiceAccountDetails>(teamsError);
        var rolesResult = await ValidateRolesAsync(unitOfWork, entitlementService, command.RoleIds, cancellationToken);
        if (!rolesResult.IsSuccess(out var roleIds, out var rolesError))
            return Result.Failure<ServiceAccountDetails>(rolesError);

        var resourceAccesses = command.ResourceAccesses?.Distinct().ToArray() ?? [];
        if (resourceAccesses.Length > 0)
        {
            var accessEntitlement = await entitlementService.EnsureEnabledAsync(LicenseCapability.CustomAccessControl, cancellationToken);
            if (accessEntitlement.IsFailure(out var accessError))
                return Result.Failure<ServiceAccountDetails>(accessError);
        }

        var now = timeProvider.GetUtcNow().UtcDateTime;
        var actor = Actor.Create(ActorType.ServiceAccount, new ActorMetadata(command.Name), command.IsEnabled);
        var account = ServiceAccount.Create(command.Name, command.Description, actor.Id, userContext.Current.ActorId, now);
        var resourceAccessEntities = resourceAccesses.Select(x => ResourceAccess.Create(
            x.ResourceType,
            x.ResourceId,
            actor.Id,
            x.PermissionLevel,
            x.SpecificPermissions)).ToArray();
        await unitOfWork.Actors.AddAsync(actor, cancellationToken);
        await unitOfWork.ServiceAccounts.AddAsync(account, cancellationToken);
        await unitOfWork.ServiceAccounts.ReplaceTeamsAsync(actor.Id, teamIds, cancellationToken);
        await unitOfWork.Roles.ReplaceActorRolesAsync(actor.Id, roleIds, cancellationToken);
        await unitOfWork.ResourceAccesses.ReplaceAsync(
            actor.Id,
            resourceAccessEntities,
            cancellationToken);
        await unitOfWork.ActivityEventRepository.AddAsync(
            ServiceAccountActivity.Create(
                account,
                userContext.Current.ActorId,
                ActivityEventType.ServiceAccountCreated,
                new ServiceAccountCreated(ServiceAccountActivity.Snapshot(
                    account,
                    actor.IsEnabled,
                    teamIds,
                    roleIds,
                    resourceAccessEntities))),
            cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);
        await evictor.EvictActorAsync(actor.Id, CancellationToken.None);

        return Result.Success((await GetServiceAccountHandler.LoadDetailsAsync(unitOfWork, account.Id, cancellationToken))!);
    }

    internal static async Task<Result<Guid[]>> ValidateTeamsAsync(
        IUnitOfWork unitOfWork,
        ILicenseEntitlementService entitlementService,
        IEnumerable<Guid>? requested,
        CancellationToken cancellationToken)
    {
        var ids = requested?.Distinct().ToArray() ?? [];
        if (ids.Length == 0)
            return Result.Success(ids);
        var teams = (await unitOfWork.Teams.GetAllAsync(ids, cancellationToken) ?? []).ToArray();
        if (teams.Length != ids.Length)
            return Result.Failure<Guid[]>(new NotFoundError("One or more selected Teams do not exist."));
        if (await unitOfWork.Actors.HasCustomAccessConfigurationAsync(teams.Select(x => x.ActorId), cancellationToken))
        {
            var result = await entitlementService.EnsureEnabledAsync(LicenseCapability.CustomAccessControl, cancellationToken);
            if (result.IsFailure(out var error))
                return Result.Failure<Guid[]>(error);
        }
        return Result.Success(ids);
    }

    internal static async Task<Result<Guid[]>> ValidateRolesAsync(
        IUnitOfWork unitOfWork,
        ILicenseEntitlementService entitlementService,
        IEnumerable<Guid>? requested,
        CancellationToken cancellationToken)
    {
        var ids = requested?.Distinct().ToArray() ?? [];
        if (ids.Length == 0)
            return Result.Success(ids);
        var roles = (await unitOfWork.Roles.GetAllAsync(ids, cancellationToken) ?? []).ToArray();
        if (roles.Length != ids.Length)
            return Result.Failure<Guid[]>(new NotFoundError("One or more selected Roles do not exist."));
        if (roles.Any(x => x.RoleType == RoleType.Custom))
        {
            var result = await entitlementService.EnsureEnabledAsync(LicenseCapability.CustomAccessControl, cancellationToken);
            if (result.IsFailure(out var error))
                return Result.Failure<Guid[]>(error);
        }
        return Result.Success(ids);
    }
}

[RequirePermission(ResourceType.ServiceAccount, PermissionLevel.Read, ResourceIdProperty = nameof(GetServiceAccountUsages.Id))]
public sealed record GetServiceAccountUsages(Guid Id)
    : IQuery<Result<IReadOnlyList<RunAsActorUsage>>>;

internal sealed class GetServiceAccountUsagesHandler(IUnitOfWork unitOfWork)
    : IQueryHandler<GetServiceAccountUsages, Result<IReadOnlyList<RunAsActorUsage>>>
{
    public async ValueTask<Result<IReadOnlyList<RunAsActorUsage>>> Handle(
        GetServiceAccountUsages query,
        CancellationToken cancellationToken)
    {
        var account = await unitOfWork.ServiceAccounts.GetAsync(query.Id, cancellationToken);
        return account is null
            ? Result.Failure<IReadOnlyList<RunAsActorUsage>>(new NotFoundError("The Service Account does not exist."))
            : Result.Success(await unitOfWork.ServiceAccounts.GetRunAsUsagesAsync(account.ActorId, cancellationToken));
    }
}

[RequirePermission(ResourceType.ServiceAccount, PermissionLevel.Read, ResourceIdProperty = nameof(GetServiceAccountTokens.Id))]
public sealed record GetServiceAccountTokens(Guid Id, int Page = 1, int PageSize = 50)
    : IQuery<Result<PagedResult<ServiceAccountTokenDetails>>>
{
    internal sealed class Validator : AbstractValidator<GetServiceAccountTokens>
    {
        public Validator()
        {
            RuleFor(x => x.Id).NotEmpty();
            RuleFor(x => x.Page).GreaterThan(0);
            RuleFor(x => x.PageSize).InclusiveBetween(1, 100);
        }
    }
}

internal sealed class GetServiceAccountTokensHandler(IUnitOfWork unitOfWork)
    : IQueryHandler<GetServiceAccountTokens, Result<PagedResult<ServiceAccountTokenDetails>>>
{
    public async ValueTask<Result<PagedResult<ServiceAccountTokenDetails>>> Handle(GetServiceAccountTokens query, CancellationToken cancellationToken)
    {
        if (await unitOfWork.ServiceAccounts.GetAsync(query.Id, cancellationToken) is null)
            return Result.Failure<PagedResult<ServiceAccountTokenDetails>>(new NotFoundError("The Service Account does not exist."));
        return Result.Success(await unitOfWork.ServiceAccounts.GetTokensAsync(query.Id, query.Page, query.PageSize, cancellationToken));
    }
}

[RequirePermission(
    ResourceType.ServiceAccount,
    PermissionLevel.Read,
    SpecificPermission.ManageCredentials,
    ResourceIdProperty = nameof(CreateServiceAccountToken.Id))]
public sealed record CreateServiceAccountToken(Guid Id, string Name, DateTimeOffset? ExpiresAtUtc, bool NeverExpires = false)
    : ICommand<Result<CreatedServiceAccountToken>>, IHumanPrincipalRequest
{
    internal sealed class Validator : AbstractValidator<CreateServiceAccountToken>
    {
        public Validator()
        {
            RuleFor(x => x.Id).NotEmpty();
            RuleFor(x => x.Name).ValidNameIdentifier();
        }
    }
}

internal sealed class CreateServiceAccountTokenHandler(
    IUnitOfWork unitOfWork,
    IUserContextAccessor userContext,
    ILicenseEntitlementService entitlementService,
    IOptions<ServiceAccountOptions> options,
    TimeProvider timeProvider)
    : ICommandHandler<CreateServiceAccountToken, Result<CreatedServiceAccountToken>>
{
    public async ValueTask<Result<CreatedServiceAccountToken>> Handle(CreateServiceAccountToken command, CancellationToken cancellationToken)
    {
        var entitlement = await entitlementService.EnsureEnabledAsync(LicenseCapability.CustomAccessControl, cancellationToken);
        if (entitlement.IsFailure(out var entitlementError))
            return Result.Failure<CreatedServiceAccountToken>(entitlementError);
        var account = await unitOfWork.ServiceAccounts.GetAsync(command.Id, cancellationToken);
        if (account is null || account.ArchivedAtUtc.HasValue)
            return Result.Failure<CreatedServiceAccountToken>(new NotFoundError("The Service Account does not exist."));
        var now = timeProvider.GetUtcNow();
        if (command.NeverExpires && command.ExpiresAtUtc.HasValue)
            return Result.Failure<CreatedServiceAccountToken>(new BadRequestError("A non-expiring token cannot also have an expiration date."));
        DateTimeOffset? expiresAtUtc = command.NeverExpires
            ? null
            : command.ExpiresAtUtc ?? now.AddDays(options.Value.DefaultTokenLifetimeDays);
        if (expiresAtUtc.HasValue && expiresAtUtc.Value <= now)
            return Result.Failure<CreatedServiceAccountToken>(new BadRequestError("Token expiration must be in the future."));
        if (expiresAtUtc.HasValue && expiresAtUtc.Value > now.AddDays(options.Value.MaximumTokenLifetimeDays))
            return Result.Failure<CreatedServiceAccountToken>(new BadRequestError($"Token expiration cannot exceed {options.Value.MaximumTokenLifetimeDays} days."));

        var credentialId = Guid.CreateVersion7();
        var secret = RandomNumberGenerator.GetBytes(32);
        var hash = SHA256.HashData(secret);
        try
        {
            var entity = ServiceAccountToken.Create(
                credentialId,
                command.Id,
                command.Name,
                hash,
                expiresAtUtc?.UtcDateTime,
                userContext.Current.ActorId,
                now.UtcDateTime);
            var insert = await unitOfWork.ServiceAccounts.TryAddTokenAsync(
                entity,
                options.Value.MaximumActiveTokensPerAccount,
                now.UtcDateTime,
                cancellationToken);
            if (insert != ServiceAccountTokenInsertResult.Created)
            {
                return insert switch
                {
                    ServiceAccountTokenInsertResult.AccountUnavailable => Result.Failure<CreatedServiceAccountToken>(new NotFoundError("The Service Account does not exist.")),
                    ServiceAccountTokenInsertResult.DuplicateName => Result.Failure<CreatedServiceAccountToken>(new ConflictError("A token with this name already exists for the Service Account.")),
                    _ => Result.Failure<CreatedServiceAccountToken>(new ConflictError("The Service Account has reached its active-token limit.")),
                };
            }

            await unitOfWork.ActivityEventRepository.AddAsync(
                ServiceAccountActivity.Create(
                    account,
                    userContext.Current.ActorId,
                    ActivityEventType.ServiceAccountTokenCreated,
                    new ServiceAccountTokenCreated(
                        account.Id,
                        entity.Id,
                        entity.Name,
                        $"cit_sa_{entity.Id:N}"[..15],
                        entity.ExpiresAtUtc)),
                cancellationToken);
            await unitOfWork.CommitAsync(cancellationToken);
            var token = $"cit_sa_{credentialId:N}.{Base64Url(secret)}";
            var creator = await unitOfWork.Actors.GetById(entity.CreatedByActorId, cancellationToken);
            var details = new ServiceAccountTokenDetails(
                entity.Id,
                entity.Name,
                entity.ExpiresAtUtc,
                null,
                null,
                null,
                entity.CreatedByActorId,
                creator?.ActorMetadata.Name ?? "Unknown",
                entity.CreatedAtUtc);
            return Result.Success(new CreatedServiceAccountToken(details, token));
        }
        finally
        {
            CryptographicOperations.ZeroMemory(secret);
            CryptographicOperations.ZeroMemory(hash);
        }
    }

    private static string Base64Url(byte[] value)
        => Convert.ToBase64String(value).TrimEnd('=').Replace('+', '-').Replace('/', '_');
}

[RequirePermission(
    ResourceType.ServiceAccount,
    PermissionLevel.Read,
    SpecificPermission.ManageCredentials,
    ResourceIdProperty = nameof(RevokeServiceAccountToken.Id))]
public sealed record RevokeServiceAccountToken(Guid Id, Guid TokenId) : ICommand<Result>, IHumanPrincipalRequest;

internal sealed class RevokeServiceAccountTokenHandler(
    IUnitOfWork unitOfWork,
    IUserContextAccessor userContext,
    TimeProvider timeProvider)
    : ICommandHandler<RevokeServiceAccountToken, Result>
{
    public async ValueTask<Result> Handle(RevokeServiceAccountToken command, CancellationToken cancellationToken)
    {
        var account = await unitOfWork.ServiceAccounts.GetAsync(command.Id, cancellationToken);
        if (account is null)
            return Result.Failure(new NotFoundError("The Service Account does not exist."));
        var affected = await unitOfWork.ServiceAccounts.RevokeTokenAsync(
            command.Id,
            command.TokenId,
            userContext.Current.ActorId,
            timeProvider.GetUtcNow().UtcDateTime,
            cancellationToken);
        if (affected > 0)
        {
            await unitOfWork.ActivityEventRepository.AddAsync(
                ServiceAccountActivity.Create(
                    account,
                    userContext.Current.ActorId,
                    ActivityEventType.ServiceAccountTokenRevoked,
                    new ServiceAccountTokenRevoked(
                        account.Id,
                        command.TokenId,
                        $"cit_sa_{command.TokenId:N}"[..15])),
                cancellationToken);
        }
        await unitOfWork.CommitAsync(cancellationToken);
        return Result.Success();
    }
}

internal static class ServiceAccountActivity
{
    internal static ActivityEvent Create(
        ServiceAccount account,
        Guid actorId,
        ActivityEventType eventType,
        ActivityEventInfo info)
        => new(
            platformId: null,
            resourceId: account.Id,
            actorId: actorId,
            resourceName: account.Name,
            eventType: eventType,
            status: ActivityStatus.Success,
            info: info);

    internal static ServiceAccountActivitySnapshot Snapshot(
        ServiceAccount account,
        bool isEnabled,
        IEnumerable<Guid> teamIds,
        IEnumerable<Guid> roleIds,
        IEnumerable<ResourceAccess> resourceAccesses)
        => new(
            account.Id,
            account.Name,
            account.Description,
            isEnabled,
            teamIds.Order().ToArray(),
            roleIds.Order().ToArray(),
            resourceAccesses
                .OrderBy(static access => access.ResourceType)
                .ThenBy(static access => access.ResourceId)
                .Select(static access => new ServiceAccountResourceAccessSnapshot(
                    access.ResourceType,
                    access.ResourceId,
                    access.PermissionLevel,
                    access.SpecificPermissions.Aggregate(0, static (mask, permission) => mask | (int)permission)))
                .ToArray());

    internal static (ActivityEventType EventType, ActivityEventInfo Info) DescribeUpdate(
        Guid accountId,
        string oldName,
        string newName,
        bool wasEnabled,
        bool isEnabled,
        ServiceAccountActivitySnapshot oldSnapshot,
        ServiceAccountActivitySnapshot newSnapshot)
    {
        var renamed = !string.Equals(oldName, newName, StringComparison.Ordinal);
        var stateChanged = wasEnabled != isEnabled;
        if (renamed && !stateChanged && SameExceptName(oldSnapshot, newSnapshot))
            return (ActivityEventType.ServiceAccountRenamed, new ServiceAccountRenamed(oldName, newName));
        if (!renamed && stateChanged && SameExceptEnabled(oldSnapshot, newSnapshot))
        {
            return isEnabled
                ? (ActivityEventType.ServiceAccountEnabled, new ServiceAccountEnabled(accountId))
                : (ActivityEventType.ServiceAccountDisabled, new ServiceAccountDisabled(accountId));
        }

        return (ActivityEventType.ServiceAccountUpdated, new ServiceAccountUpdated(oldSnapshot, newSnapshot));
    }

    private static bool SameExceptName(ServiceAccountActivitySnapshot left, ServiceAccountActivitySnapshot right)
        => left.Description == right.Description
           && left.IsEnabled == right.IsEnabled
           && left.TeamIds.SequenceEqual(right.TeamIds)
           && left.RoleIds.SequenceEqual(right.RoleIds)
           && left.ResourceAccesses.SequenceEqual(right.ResourceAccesses);

    private static bool SameExceptEnabled(ServiceAccountActivitySnapshot left, ServiceAccountActivitySnapshot right)
        => left.Name == right.Name
           && left.Description == right.Description
           && left.TeamIds.SequenceEqual(right.TeamIds)
           && left.RoleIds.SequenceEqual(right.RoleIds)
           && left.ResourceAccesses.SequenceEqual(right.ResourceAccesses);
}

public sealed record GetServiceAccountLimits : IQuery<Result<ServiceAccountLimits>>;

internal sealed class GetServiceAccountLimitsHandler(IOptions<ServiceAccountOptions> options)
    : IQueryHandler<GetServiceAccountLimits, Result<ServiceAccountLimits>>
{
    public ValueTask<Result<ServiceAccountLimits>> Handle(GetServiceAccountLimits query, CancellationToken cancellationToken)
        => ValueTask.FromResult(Result.Success(new ServiceAccountLimits(
            options.Value.DefaultTokenLifetimeDays,
            options.Value.MaximumTokenLifetimeDays,
            options.Value.MaximumActiveTokensPerAccount)));
}
