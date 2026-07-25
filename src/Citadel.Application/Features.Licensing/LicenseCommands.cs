using Application.Services;
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
    ILicenseStateProvider licenseStateProvider) : ICommandHandler<InstallLicense, Result<LicenseState>>
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

        var existing = await unitOfWork.InstalledLicense.GetAsync(cancellationToken);
        var oldSnapshot = existing is null
            ? null
            : ToSnapshot(verifier.Verify(existing.RawLicense, identity, now), existing.Fingerprint);

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
        var state = await LicenseStateBuilder.BuildAsync(unitOfWork, verifier, timeProvider, cancellationToken, identity);

        await unitOfWork.CommitAsync(cancellationToken);
        await licenseStateProvider.ReloadAsync(cancellationToken);

        return state;
    }

    private static LicenseActivitySnapshot ToSnapshot(LicenseVerificationResult verification, string? fallbackFingerprint)
    {
        if (verification.License is null)
        {
            return new LicenseActivitySnapshot(
                LicenseId: null,
                ReplacedLicenseId: null,
                Edition: LicenseConstants.EditionCommunity,
                CustomerId: null,
                CustomerName: null,
                Fingerprint: fallbackFingerprint,
                Status: verification.Status,
                ExpiresAt: null,
                GraceUntil: null);
        }

        var payload = verification.License.Payload;
        return new LicenseActivitySnapshot(
            LicenseId: payload.LicenseId,
            ReplacedLicenseId: payload.ReplacedLicenseId,
            Edition: payload.Edition,
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
    ILicenseStateProvider licenseStateProvider) : ICommandHandler<RemoveLicense, Result<LicenseState>>
{
    public async ValueTask<Result<LicenseState>> Handle(RemoveLicense command, CancellationToken cancellationToken)
    {
        var existing = await unitOfWork.InstalledLicense.GetAsync(cancellationToken);
        if (existing is null)
            return await licenseStateProvider.GetCurrentAsync(cancellationToken);

        var now = timeProvider.GetUtcNow();
        var identity = await unitOfWork.InstanceIdentity.GetOrCreateAsync(Guid.CreateVersion7(), now, cancellationToken);
        var snapshot = InstallLicenseHandlerToSnapshot(verifier.Verify(existing.RawLicense, identity, now), existing.Fingerprint);

        await unitOfWork.InstalledLicense.DeleteAsync(cancellationToken);
        var state = await LicenseStateBuilder.BuildAsync(unitOfWork, verifier, timeProvider, cancellationToken, identity);

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

        return state;
    }

    private static LicenseActivitySnapshot InstallLicenseHandlerToSnapshot(
        LicenseVerificationResult verification,
        string? fallbackFingerprint)
    {
        if (verification.License is null)
        {
            return new LicenseActivitySnapshot(
                LicenseId: null,
                ReplacedLicenseId: null,
                Edition: LicenseConstants.EditionCommunity,
                CustomerId: null,
                CustomerName: null,
                Fingerprint: fallbackFingerprint,
                Status: verification.Status,
                ExpiresAt: null,
                GraceUntil: null);
        }

        var payload = verification.License.Payload;
        return new LicenseActivitySnapshot(
            LicenseId: payload.LicenseId,
            ReplacedLicenseId: payload.ReplacedLicenseId,
            Edition: payload.Edition,
            CustomerId: payload.Customer.Id,
            CustomerName: payload.Customer.Name,
            Fingerprint: verification.License.Fingerprint,
            Status: verification.Status,
            ExpiresAt: payload.ExpiresAt,
            GraceUntil: payload.GraceUntil);
    }
}
