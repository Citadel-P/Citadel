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

public interface ILicenseQuotaService
{
    ValueTask<LicenseState> GetOverviewAsync(CancellationToken cancellationToken);

    ValueTask<Result> EnsureCanIncreaseAsync(
        IReadOnlyDictionary<LicenseLimit, int> increases,
        IUnitOfWork unitOfWork,
        CancellationToken cancellationToken);
}

public sealed class LicenseStateProvider(
    IServiceScopeFactory scopeFactory,
    TimeProvider timeProvider,
    ILicenseVerifier verifier) : ILicenseStateProvider
{
    public async ValueTask<LicenseState> GetCurrentAsync(CancellationToken cancellationToken)
    {
        await using var scope = scopeFactory.CreateAsyncScope();
        await using var unitOfWork = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var state = await LicenseStateBuilder.BuildAsync(unitOfWork, verifier, timeProvider, cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);
        return state;
    }

    public ValueTask ReloadAsync(CancellationToken cancellationToken)
        => ValueTask.CompletedTask;
}

public sealed class LicenseQuotaService(
    IServiceScopeFactory scopeFactory,
    TimeProvider timeProvider,
    ILicenseVerifier verifier) : ILicenseQuotaService
{
    public async ValueTask<LicenseState> GetOverviewAsync(CancellationToken cancellationToken)
    {
        await using var scope = scopeFactory.CreateAsyncScope();
        await using var unitOfWork = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var state = await LicenseStateBuilder.BuildAsync(unitOfWork, verifier, timeProvider, cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);
        return state;
    }

    public async ValueTask<Result> EnsureCanIncreaseAsync(
        IReadOnlyDictionary<LicenseLimit, int> increases,
        IUnitOfWork unitOfWork,
        CancellationToken cancellationToken)
    {
        if (increases.Count == 0 || increases.All(x => x.Value <= 0))
            return Result.Success();

        var now = timeProvider.GetUtcNow();
        var identity = await unitOfWork.InstanceIdentity.GetOrCreateLockedAsync(Guid.CreateVersion7(), now, cancellationToken);

        var state = await LicenseStateBuilder.BuildAsync(unitOfWork, verifier, timeProvider, cancellationToken, identity);
        var violations = new List<LicenseQuotaViolation>();

        foreach (var (limit, delta) in increases)
        {
            if (delta <= 0)
                continue;

            var current = state.Usage.GetValue(limit);
            var maximum = state.EffectiveLimits[limit];
            if (current + delta > maximum)
            {
                violations.Add(new LicenseQuotaViolation(
                    Limit: limit,
                    Current: current,
                    Requested: delta,
                    Maximum: maximum));
            }
        }

        if (violations.Count == 0)
            return Result.Success();

        var metadata = new Dictionary<string, object>
        {
            ["violations"] = violations.ToArray(),
            ["licenseStatus"] = state.Status.ToString(),
            ["edition"] = state.Edition
        };

        return Result.Failure(new ForbiddenError("License quota exceeded.", metadata));
    }
}

internal static class LicenseStateBuilder
{
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
        var readModel = await unitOfWork.LicenseUsage.GetLicenseReadModelAsync(cancellationToken);
        var usage = readModel.Usage;
        var installed = readModel.InstalledLicense;

        if (installed is null)
            return Community(identity.InstanceId, usage);

        var verification = verifier.Verify(installed.RawLicense, identity, now);
        if (verification.License is null)
        {
            return new LicenseState(
                Status: verification.Status,
                Edition: LicenseConstants.EditionCommunity,
                InstanceId: identity.InstanceId,
                LicenseId: null,
                ReplacedLicenseId: null,
                CustomerId: null,
                CustomerName: null,
                Fingerprint: installed.Fingerprint,
                IssuedAt: null,
                NotBefore: null,
                ExpiresAt: null,
                GraceUntil: null,
                EffectiveLimits: CommunityLicenseLimits.Values,
                Usage: usage,
                Warnings: verification.ErrorMessage is null ? [] : [verification.ErrorMessage]);
        }

        var license = verification.License;
        var payload = license.Payload;
        var useSignedLimits = verification.Status is LicenseStatus.Valid or LicenseStatus.GracePeriod;
        return new LicenseState(
            Status: verification.Status,
            Edition: useSignedLimits ? payload.Edition : LicenseConstants.EditionCommunity,
            InstanceId: identity.InstanceId,
            LicenseId: payload.LicenseId,
            ReplacedLicenseId: payload.ReplacedLicenseId,
            CustomerId: payload.Customer.Id,
            CustomerName: payload.Customer.Name,
            Fingerprint: license.Fingerprint,
            IssuedAt: payload.IssuedAt,
            NotBefore: payload.NotBefore,
            ExpiresAt: payload.ExpiresAt,
            GraceUntil: payload.GraceUntil,
            EffectiveLimits: useSignedLimits ? license.EffectiveLimits : CommunityLicenseLimits.Values,
            Usage: usage,
            Warnings: license.Warnings);
    }

    private static LicenseState Community(Guid instanceId, LicenseUsageSnapshot usage)
        => new(
            Status: LicenseStatus.Community,
            Edition: LicenseConstants.EditionCommunity,
            InstanceId: instanceId,
            LicenseId: null,
            ReplacedLicenseId: null,
            CustomerId: null,
            CustomerName: null,
            Fingerprint: null,
            IssuedAt: null,
            NotBefore: null,
            ExpiresAt: null,
            GraceUntil: null,
            EffectiveLimits: CommunityLicenseLimits.Values,
            Usage: usage,
            Warnings: []);
}
