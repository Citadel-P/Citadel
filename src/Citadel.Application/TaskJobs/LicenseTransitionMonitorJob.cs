using Application.Services.Alerts;
using Application.Services.Licensing;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Activities;
using Domain.Entities.Licensing;
using Hosting.Common;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Hosting;
using Microsoft.Extensions.Logging;

namespace Application.TaskJobs;

internal sealed class LicenseTransitionMonitorJob(
    IServiceScopeFactory scopeFactory,
    TimeProvider timeProvider,
    ILicenseVerifier verifier,
    IAlertService alertService,
    ILogger<LicenseTransitionMonitorJob> logger) : BackgroundService
{
    private static readonly TimeSpan CheckInterval = TimeSpan.FromHours(1);

    protected override async Task ExecuteAsync(CancellationToken stoppingToken)
    {
        await CheckLicenseAsync(stoppingToken);

        using var timer = new PeriodicTimer(CheckInterval);
        while (await timer.WaitForNextTickAsync(stoppingToken))
        {
            await CheckLicenseAsync(stoppingToken);
        }
    }

    private async Task CheckLicenseAsync(CancellationToken cancellationToken)
    {
        try
        {
            await using var scope = scopeFactory.CreateAsyncScope();
            var unitOfWork = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            var installed = await unitOfWork.InstalledLicense.GetAsync(cancellationToken);
            if (installed is null)
                return;

            var now = timeProvider.GetUtcNow();
            var identity = await unitOfWork.InstanceIdentity.GetOrCreateAsync(Guid.CreateVersion7(), now, cancellationToken);
            var verification = verifier.Verify(installed.RawLicense, identity, now);
            var previousStatus = installed.LastValidationStatus;
            AlertType? alertType = null;
            LicenseAlertSnapshot? alertSnapshot = null;

            await unitOfWork.InstalledLicense.UpdateValidationStatusAsync(
                verification.Status,
                now,
                verification.ErrorCode,
                cancellationToken);

            if (previousStatus != verification.Status)
            {
                var activity = BuildTransitionActivity(identity, installed, verification);
                if (activity is not null)
                {
                    await unitOfWork.ActivityEventRepository.AddAsync(activity, cancellationToken);
                    alertType = GetAlertType(verification.Status);
                    alertSnapshot = ToAlertSnapshot(identity, installed, verification);
                }
            }

            await unitOfWork.CommitAsync(cancellationToken);

            if (alertType is not null && alertSnapshot is not null)
                await DispatchAlertAsync(alertType.Value, alertSnapshot, now, cancellationToken);
        }
        catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested)
        {
        }
        catch (Exception ex)
        {
            logger.LogError(ex, "Error occurred while monitoring license state.");
        }
    }

    private static ActivityEvent? BuildTransitionActivity(
        CitadelInstanceIdentity identity,
        InstalledLicense installed,
        LicenseVerificationResult verification)
    {
        var snapshot = ToSnapshot(verification, installed.Fingerprint);

        return verification.Status switch
        {
            LicenseStatus.GracePeriod => Activity(ActivityEventType.LicenseEnteredGracePeriod, new LicenseEnteredGracePeriod(snapshot)),
            LicenseStatus.Expired => Activity(ActivityEventType.LicenseExpired, new LicenseExpired(snapshot)),
            LicenseStatus.Invalid or LicenseStatus.InstanceMismatch or LicenseStatus.UnsupportedSchema or LicenseStatus.UnknownSigningKey
                => Activity(
                    ActivityEventType.LicenseValidationFailed,
                    new LicenseValidationFailed(installed.Fingerprint, verification.Status, verification.ErrorCode)),
            _ => null
        };

        ActivityEvent Activity(ActivityEventType type, ActivityEventInfo info)
            => new(
                platformId: null,
                resourceId: identity.InstanceId,
                actorId: Constants.SystemId,
                resourceName: "License",
                eventType: type,
                status: ActivityStatus.Success,
                info: info);
    }

    private static AlertType? GetAlertType(LicenseStatus status)
        => status switch
        {
            LicenseStatus.GracePeriod => AlertType.LicenseEnteredGracePeriod,
            LicenseStatus.Expired => AlertType.LicenseExpired,
            _ => null
        };

    private static LicenseAlertSnapshot ToAlertSnapshot(
        CitadelInstanceIdentity identity,
        InstalledLicense installed,
        LicenseVerificationResult verification)
    {
        var payload = verification.License?.Payload;
        return new LicenseAlertSnapshot(
            InstanceId: identity.InstanceId,
            LicenseId: payload?.LicenseId,
            CustomerName: payload?.Customer.Name,
            Fingerprint: verification.License?.Fingerprint ?? installed.Fingerprint,
            Status: verification.Status,
            ExpiresAt: payload?.ExpiresAt,
            GraceUntil: payload?.GraceUntil);
    }

    private Task DispatchAlertAsync(
        AlertType type,
        LicenseAlertSnapshot snapshot,
        DateTimeOffset utcNow,
        CancellationToken cancellationToken)
        => alertService.ProcessAsync(
            type,
            new AlertEvaluationContext(
                UtcNow: utcNow.UtcDateTime,
                Platforms: [],
                Deployments: [],
                Stacks: [],
                Licenses: [snapshot]),
            cancellationToken);

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
