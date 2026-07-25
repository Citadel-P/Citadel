using Application.Services;
using Application.Services.Abstractions;
using Application.Services.Licensing;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Activities;
using Domain.Entities.Licensing;
using Hosting.Common;
using Hosting.Common.Abstraction;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.Licensing;

[RequirePermission(ResourceType.License, PermissionLevel.Read)]
public sealed record GetLicense : IQuery<Result<LicenseState>>;

public sealed record GetLicenseEntitlements : IQuery<Result<LicenseState>>;

[RequirePermission(ResourceType.License, PermissionLevel.Read)]
public sealed record GetLicenseRequest : IQuery<Result<LicenseRequest>>;

[RequirePermission(ResourceType.License, PermissionLevel.Write)]
public sealed record InstallLicense(string License) : ICommand<Result<LicenseState>>;

[RequirePermission(ResourceType.License, PermissionLevel.Execute)]
public sealed record RemoveLicense : ICommand<Result<LicenseState>>;

public sealed record LicenseRequest(
    string Product,
    Guid InstanceId,
    string CoreVersion,
    DateTimeOffset GeneratedAt);

internal sealed class GetLicenseHandler(ILicenseStateProvider licenseStateProvider)
    : IQueryHandler<GetLicense, Result<LicenseState>>
{
    public async ValueTask<Result<LicenseState>> Handle(GetLicense query, CancellationToken cancellationToken)
        => await licenseStateProvider.GetCurrentAsync(cancellationToken);
}

internal sealed class GetLicenseEntitlementsHandler(ILicenseEntitlementService entitlementService)
    : IQueryHandler<GetLicenseEntitlements, Result<LicenseState>>
{
    public async ValueTask<Result<LicenseState>> Handle(
        GetLicenseEntitlements query,
        CancellationToken cancellationToken)
        => await entitlementService.GetOverviewAsync(cancellationToken);
}

internal sealed class GetLicenseRequestHandler(IUnitOfWork unitOfWork, TimeProvider timeProvider)
    : IQueryHandler<GetLicenseRequest, Result<LicenseRequest>>
{
    public async ValueTask<Result<LicenseRequest>> Handle(GetLicenseRequest query, CancellationToken cancellationToken)
    {
        var now = timeProvider.GetUtcNow();
        var identity = await unitOfWork.InstanceIdentity.GetOrCreateAsync(Guid.CreateVersion7(), now, cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);
        return new LicenseRequest(
            Product: LicenseConstants.Product,
            InstanceId: identity.InstanceId,
            CoreVersion: Constants.CompatibilityVersion,
            GeneratedAt: now);
    }
}

internal sealed class InstallLicenseHandler(
    IUnitOfWork unitOfWork,
    TimeProvider timeProvider,
    IUserContextAccessor userContext,
    ILicenseVerifier verifier,
    ILicenseStateProvider licenseStateProvider,
    IApplicationHubDispatcher hubDispatcher) : ICommandHandler<InstallLicense, Result<LicenseState>>
{
    public async ValueTask<Result<LicenseState>> Handle(InstallLicense command, CancellationToken cancellationToken)
    {
        var now = timeProvider.GetUtcNow();
        var identity = await unitOfWork.InstanceIdentity.GetOrCreateAsync(Guid.CreateVersion7(), now, cancellationToken);

        if (!LicenseInputNormalizer.TryNormalize(command.License, out var normalized, out var normalizeError))
            return Result.Failure<LicenseState>(new BadRequestError(normalizeError));

        var verification = verifier.Verify(normalized, identity, now);
        if (!verification.IsAccepted || verification.License is null)
            return Result.Failure<LicenseState>(new BadRequestError(verification.ErrorMessage ?? "License is not valid."));

        var existing = await unitOfWork.InstalledLicense.GetLockedAsync(cancellationToken);
        LicenseVerificationResult? existingVerification = null;
        if (existing is not null)
        {
            existingVerification = verifier.Verify(existing.RawLicense, identity, now);
            var existingLicenseId = existingVerification.License?.Payload.LicenseId;
            if (string.IsNullOrWhiteSpace(existingLicenseId)
                || !string.Equals(
                    verification.License.Payload.ReplacedLicenseId,
                    existingLicenseId,
                    StringComparison.Ordinal))
            {
                return Result.Failure<LicenseState>(ReplacementConflict(
                    "license-replacement-mismatch",
                    "The submitted license does not replace the currently installed license."));
            }

            if (verification.Status == LicenseStatus.NotYetValid
                && existingVerification.Status is LicenseStatus.Valid or LicenseStatus.GracePeriod)
            {
                return Result.Failure<LicenseState>(ReplacementConflict(
                    "license-replacement-not-yet-effective",
                    "A future-dated license cannot replace a currently active license."));
            }
        }

        var oldSnapshot = existing is null
            ? null
            : ToSnapshot(existingVerification!, existing.Fingerprint);

        var installed = new InstalledLicense(
            RawLicense: verification.License.RawLicense,
            Fingerprint: verification.License.Fingerprint,
            InstalledAt: now,
            InstalledByActorId: userContext.Current.ActorId,
            LastValidatedAt: now,
            LastValidationStatus: verification.Status,
            LastValidationErrorCode: null);

        await unitOfWork.InstalledLicense.UpsertAsync(installed, cancellationToken);

        var newSnapshot = ToSnapshot(verification, verification.License.Fingerprint);
        var activity = new ActivityEvent(
            platformId: null,
            resourceId: identity.InstanceId,
            actorId: userContext.Current.ActorId,
            resourceName: "License",
            eventType: existing is null ? ActivityEventType.LicenseInstalled : ActivityEventType.LicenseReplaced,
            status: ActivityStatus.Success,
            info: existing is null
                ? new LicenseInstalled(newSnapshot)
                : new LicenseReplaced(oldSnapshot!, newSnapshot));

        await unitOfWork.ActivityEventRepository.AddAsync(activity, cancellationToken);
        var state = LicenseStateBuilder.Build(identity, installed, verification, now);

        await unitOfWork.CommitAsync(cancellationToken);
        await licenseStateProvider.ReloadAsync(cancellationToken);
        await hubDispatcher.SendLicenseStateChanged(cancellationToken);

        return state;
    }

    private static ConflictError ReplacementConflict(string problemType, string message)
        => new(
            message,
            new Dictionary<string, object>
            {
                ["problemType"] = $"https://citadel.local/problems/{problemType}"
            });

    private static LicenseActivitySnapshot ToSnapshot(LicenseVerificationResult verification, string? fallbackFingerprint)
    {
        if (verification.License is null)
        {
            return new LicenseActivitySnapshot(
                Schema: null,
                LicenseId: null,
                ReplacedLicenseId: null,
                LicensedEdition: null,
                EffectiveEdition: LicenseConstants.EditionCommunity,
                EffectiveCapabilities: [],
                CustomerId: null,
                CustomerName: null,
                Fingerprint: fallbackFingerprint,
                Status: verification.Status,
                ExpiresAt: null,
                GraceUntil: null);
        }

        var payload = verification.License.Payload;
        var isEffective = verification.Status is LicenseStatus.Valid or LicenseStatus.GracePeriod;
        return new LicenseActivitySnapshot(
            Schema: payload.Schema,
            LicenseId: payload.LicenseId,
            ReplacedLicenseId: payload.ReplacedLicenseId,
            LicensedEdition: payload.Edition,
            EffectiveEdition: isEffective
                ? payload.Schema == LicenseConstants.LegacySchema
                    ? LicenseConstants.EditionTeam
                    : payload.Edition
                : LicenseConstants.EditionCommunity,
            EffectiveCapabilities: isEffective
                ? [.. verification.License.EffectiveCapabilities.Order()]
                : [],
            CustomerId: payload.Customer.Id,
            CustomerName: payload.Customer.Name,
            Fingerprint: verification.License.Fingerprint,
            Status: verification.Status,
            ExpiresAt: payload.ExpiresAt,
            GraceUntil: payload.GraceUntil);
    }
}

internal sealed class RemoveLicenseHandler(
    IUnitOfWork unitOfWork,
    TimeProvider timeProvider,
    IUserContextAccessor userContext,
    ILicenseVerifier verifier,
    ILicenseStateProvider licenseStateProvider,
    IApplicationHubDispatcher hubDispatcher) : ICommandHandler<RemoveLicense, Result<LicenseState>>
{
    public async ValueTask<Result<LicenseState>> Handle(RemoveLicense command, CancellationToken cancellationToken)
    {
        var existing = await unitOfWork.InstalledLicense.GetLockedAsync(cancellationToken);
        if (existing is null)
        {
            await unitOfWork.CommitAsync(cancellationToken);
            await licenseStateProvider.ReloadAsync(cancellationToken);
            return await licenseStateProvider.GetCurrentAsync(cancellationToken);
        }

        var now = timeProvider.GetUtcNow();
        var identity = await unitOfWork.InstanceIdentity.GetOrCreateAsync(Guid.CreateVersion7(), now, cancellationToken);
        var snapshot = InstallLicenseHandlerToSnapshot(verifier.Verify(existing.RawLicense, identity, now), existing.Fingerprint);

        await unitOfWork.InstalledLicense.DeleteAsync(cancellationToken);
        var state = LicenseStateBuilder.Build(identity, installed: null, verification: null, now);

        var activity = new ActivityEvent(
            platformId: null,
            resourceId: identity.InstanceId,
            actorId: userContext.Current.ActorId,
            resourceName: "License",
            eventType: ActivityEventType.LicenseRemoved,
            status: ActivityStatus.Success,
            info: new LicenseRemoved(snapshot));

        await unitOfWork.ActivityEventRepository.AddAsync(activity, cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);
        await licenseStateProvider.ReloadAsync(cancellationToken);
        await hubDispatcher.SendLicenseStateChanged(cancellationToken);

        return state;
    }

    private static LicenseActivitySnapshot InstallLicenseHandlerToSnapshot(
        LicenseVerificationResult verification,
        string? fallbackFingerprint)
    {
        if (verification.License is null)
        {
            return new LicenseActivitySnapshot(
                Schema: null,
                LicenseId: null,
                ReplacedLicenseId: null,
                LicensedEdition: null,
                EffectiveEdition: LicenseConstants.EditionCommunity,
                EffectiveCapabilities: [],
                CustomerId: null,
                CustomerName: null,
                Fingerprint: fallbackFingerprint,
                Status: verification.Status,
                ExpiresAt: null,
                GraceUntil: null);
        }

        var payload = verification.License.Payload;
        var isEffective = verification.Status is LicenseStatus.Valid or LicenseStatus.GracePeriod;
        return new LicenseActivitySnapshot(
            Schema: payload.Schema,
            LicenseId: payload.LicenseId,
            ReplacedLicenseId: payload.ReplacedLicenseId,
            LicensedEdition: payload.Edition,
            EffectiveEdition: isEffective
                ? payload.Schema == LicenseConstants.LegacySchema
                    ? LicenseConstants.EditionTeam
                    : payload.Edition
                : LicenseConstants.EditionCommunity,
            EffectiveCapabilities: isEffective
                ? [.. verification.License.EffectiveCapabilities.Order()]
                : [],
            CustomerId: payload.Customer.Id,
            CustomerName: payload.Customer.Name,
            Fingerprint: verification.License.Fingerprint,
            Status: verification.Status,
            ExpiresAt: payload.ExpiresAt,
            GraceUntil: payload.GraceUntil);
    }
}
