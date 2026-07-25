using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Licensing;
using Hosting.Common.ErrorTypes;
using LightResults;
using Microsoft.Extensions.DependencyInjection;

namespace Application.Services.Licensing;

public interface ILicenseStateProvider
{
    ValueTask<LicenseState> GetCurrentAsync(CancellationToken cancellationToken);
    ValueTask ReloadAsync(CancellationToken cancellationToken);
}

public interface ILicenseEntitlementService
{
    ValueTask<LicenseState> GetOverviewAsync(CancellationToken cancellationToken);
    ValueTask<bool> IsEnabledAsync(
        LicenseCapability capability,
        CancellationToken cancellationToken);
    ValueTask<Result> EnsureEnabledAsync(
        LicenseCapability capability,
        CancellationToken cancellationToken);
}

public sealed class LicenseStateProvider(
    IServiceScopeFactory scopeFactory,
    TimeProvider timeProvider,
    ILicenseVerifier verifier) : ILicenseStateProvider
{
    private static readonly TimeSpan SourceCacheDuration = TimeSpan.FromHours(1);
    private readonly SemaphoreSlim refreshLock = new(1, 1);
    private CachedLicenseSource? cachedSource;

    public async ValueTask<LicenseState> GetCurrentAsync(CancellationToken cancellationToken)
    {
        var now = timeProvider.GetUtcNow();
        var source = Volatile.Read(ref cachedSource);
        if (source is null || now >= source.RefreshAfter)
            source = await RefreshSourceAsync(now, cancellationToken);

        return LicenseStateBuilder.Build(
            source.Identity,
            source.InstalledLicense,
            source.Verification,
            now);
    }

    public async ValueTask ReloadAsync(CancellationToken cancellationToken)
    {
        await refreshLock.WaitAsync(cancellationToken);
        try
        {
            Volatile.Write(ref cachedSource, null);
        }
        finally
        {
            refreshLock.Release();
        }
    }

    private async ValueTask<CachedLicenseSource> RefreshSourceAsync(
        DateTimeOffset now,
        CancellationToken cancellationToken)
    {
        await refreshLock.WaitAsync(cancellationToken);
        try
        {
            var current = Volatile.Read(ref cachedSource);
            if (current is not null && now < current.RefreshAfter)
                return current;

            await using var scope = scopeFactory.CreateAsyncScope();
            await using var unitOfWork = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            var identity = await unitOfWork.InstanceIdentity.GetOrCreateAsync(
                Guid.CreateVersion7(),
                now,
                cancellationToken);
            var installed = await unitOfWork.InstalledLicense.GetAsync(cancellationToken);
            await unitOfWork.CommitAsync(cancellationToken);
            var verification = installed is null
                ? null
                : verifier.Verify(installed.RawLicense, identity, now);

            var refreshed = new CachedLicenseSource(
                identity,
                installed,
                verification,
                now + SourceCacheDuration);
            Volatile.Write(ref cachedSource, refreshed);
            return refreshed;
        }
        finally
        {
            refreshLock.Release();
        }
    }

    private sealed record CachedLicenseSource(
        CitadelInstanceIdentity Identity,
        InstalledLicense? InstalledLicense,
        LicenseVerificationResult? Verification,
        DateTimeOffset RefreshAfter);
}

public sealed class LicenseEntitlementService(
    ILicenseStateProvider stateProvider) : ILicenseEntitlementService
{
    public ValueTask<LicenseState> GetOverviewAsync(CancellationToken cancellationToken)
        => stateProvider.GetCurrentAsync(cancellationToken);

    public async ValueTask<bool> IsEnabledAsync(
        LicenseCapability capability,
        CancellationToken cancellationToken)
    {
        var state = await stateProvider.GetCurrentAsync(cancellationToken);
        return state.EffectiveCapabilities.Contains(capability);
    }

    public async ValueTask<Result> EnsureEnabledAsync(
        LicenseCapability capability,
        CancellationToken cancellationToken)
    {
        var state = await stateProvider.GetCurrentAsync(cancellationToken);
        if (state.EffectiveCapabilities.Contains(capability))
            return Result.Success();

        var metadata = new Dictionary<string, object>
        {
            ["problemType"] = "https://citadel.local/problems/license-capability-required",
            ["capability"] = capability.ToString(),
            ["licenseStatus"] = state.Status.ToString(),
            ["effectiveEdition"] = state.EffectiveEdition,
            ["licensedEdition"] = state.LicensedEdition!
        };

        return Result.Failure(
            new ForbiddenError(
                $"{GetCapabilityName(capability)} requires a {LicenseConstants.EditionTeam} license.",
                metadata));
    }

    private static string GetCapabilityName(LicenseCapability capability)
        => capability switch
        {
            LicenseCapability.CustomAccessControl => "Custom access control",
            LicenseCapability.AutomatedOperations => "Automated operations",
            LicenseCapability.AdvancedAlerting => "Advanced alerting",
            LicenseCapability.OperationalGuardrails => "Operational guardrails",
            LicenseCapability.ElasticBuildExecution => "Elastic build execution",
            _ => capability.ToString()
        };
}

internal static class LicenseStateBuilder
{
    private static readonly IReadOnlySet<LicenseCapability> CommunityCapabilities =
        new HashSet<LicenseCapability>();

    public static async ValueTask<LicenseState> BuildAsync(
        IUnitOfWork unitOfWork,
        ILicenseVerifier verifier,
        TimeProvider timeProvider,
        CancellationToken cancellationToken,
        CitadelInstanceIdentity? existingIdentity = null)
    {
        var now = timeProvider.GetUtcNow();
        var identity = existingIdentity
            ?? await unitOfWork.InstanceIdentity.GetOrCreateAsync(Guid.CreateVersion7(), now, cancellationToken);
        var installed = await unitOfWork.InstalledLicense.GetAsync(cancellationToken);
        var verification = installed is null
            ? null
            : verifier.Verify(installed.RawLicense, identity, now);
        return Build(identity, installed, verification, now);
    }

    public static LicenseState Build(
        CitadelInstanceIdentity identity,
        InstalledLicense? installed,
        LicenseVerificationResult? verification,
        DateTimeOffset now)
    {
        if (installed is null)
            return Community(identity.InstanceId);

        if (verification?.License is null)
        {
            return new LicenseState(
                Status: verification?.Status ?? LicenseStatus.Invalid,
                EffectiveEdition: LicenseConstants.EditionCommunity,
                LicensedEdition: null,
                InstanceId: identity.InstanceId,
                LicenseSchema: null,
                LicenseId: null,
                ReplacedLicenseId: null,
                CustomerId: null,
                CustomerName: null,
                Fingerprint: installed.Fingerprint,
                IssuedAt: null,
                NotBefore: null,
                ExpiresAt: null,
                GraceUntil: null,
                EffectiveCapabilities: CommunityCapabilities,
                Warnings: verification?.ErrorMessage is null ? [] : [verification.ErrorMessage]);
        }

        var license = verification.License;
        var payload = license.Payload;
        var currentStatus = LicenseVerifier.DeriveTemporalStatus(payload, now);
        var paidCapabilitiesEffective = currentStatus is LicenseStatus.Valid or LicenseStatus.GracePeriod;
        var effectiveEdition = paidCapabilitiesEffective
            ? payload.Schema == LicenseConstants.LegacySchema
                ? LicenseConstants.EditionTeam
                : payload.Edition
            : LicenseConstants.EditionCommunity;

        return new LicenseState(
            Status: currentStatus,
            EffectiveEdition: effectiveEdition,
            LicensedEdition: payload.Edition,
            InstanceId: identity.InstanceId,
            LicenseSchema: payload.Schema,
            LicenseId: payload.LicenseId,
            ReplacedLicenseId: payload.ReplacedLicenseId,
            CustomerId: payload.Customer.Id,
            CustomerName: payload.Customer.Name,
            Fingerprint: license.Fingerprint,
            IssuedAt: payload.IssuedAt,
            NotBefore: payload.NotBefore,
            ExpiresAt: payload.ExpiresAt,
            GraceUntil: payload.GraceUntil,
            EffectiveCapabilities: paidCapabilitiesEffective
                ? license.EffectiveCapabilities
                : CommunityCapabilities,
            Warnings: license.Warnings);
    }

    private static LicenseState Community(Guid instanceId)
        => new(
            Status: LicenseStatus.Community,
            EffectiveEdition: LicenseConstants.EditionCommunity,
            LicensedEdition: null,
            InstanceId: instanceId,
            LicenseSchema: null,
            LicenseId: null,
            ReplacedLicenseId: null,
            CustomerId: null,
            CustomerName: null,
            Fingerprint: null,
            IssuedAt: null,
            NotBefore: null,
            ExpiresAt: null,
            GraceUntil: null,
            EffectiveCapabilities: CommunityCapabilities,
            Warnings: []);
}
