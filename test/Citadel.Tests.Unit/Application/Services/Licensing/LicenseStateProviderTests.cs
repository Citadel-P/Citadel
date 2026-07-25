using Application.Services.Licensing;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Licensing;
using Microsoft.Extensions.DependencyInjection;
using Moq;

namespace Tests.Unit.Application.Services.Licensing;

public sealed class LicenseStateProviderTests
{
    [Fact]
    public async Task GetCurrentAsync_Should_Cache_Static_Verification_And_Recalculate_Temporal_Status()
    {
        var now = new DateTimeOffset(2026, 7, 25, 10, 0, 0, TimeSpan.Zero);
        var timeProvider = new MutableTimeProvider(now);
        var identity = new CitadelInstanceIdentity(
            Guid.Parse("11111111-1111-1111-1111-111111111111"),
            now.AddDays(-1));
        var payload = new LicensePayload(
            Schema: LicenseConstants.CurrentSchema,
            Product: LicenseConstants.Product,
            Issuer: LicenseConstants.Issuer,
            Audience: LicenseConstants.Audience,
            LicenseId: "lic-state-provider-test",
            ReplacedLicenseId: null,
            Customer: new LicenseCustomer("customer-test", "Test Customer"),
            Edition: LicenseConstants.EditionTeam,
            InstanceId: identity.InstanceId,
            IssuedAt: now.AddDays(-1),
            NotBefore: now.AddDays(-1),
            ExpiresAt: now.AddMinutes(5),
            GraceUntil: null,
            Limits: null,
            Capabilities: [LicenseCapabilityKeys.AutomatedOperations]);
        var verifiedLicense = new VerifiedLicense(
            RawLicense: "header.payload.signature",
            Fingerprint: "fingerprint",
            KeyId: "test-key",
            Payload: payload,
            Status: LicenseStatus.Valid,
            EffectiveCapabilities: new HashSet<LicenseCapability> { LicenseCapability.AutomatedOperations },
            Warnings: []);
        var installed = new InstalledLicense(
            verifiedLicense.RawLicense,
            verifiedLicense.Fingerprint,
            now,
            null);

        var identityRepository = new Mock<IInstanceIdentityRepository>(MockBehavior.Strict);
        identityRepository
            .Setup(x => x.GetOrCreateAsync(It.IsAny<Guid>(), now, It.IsAny<CancellationToken>()))
            .ReturnsAsync(identity);
        var installedLicenseRepository = new Mock<IInstalledLicenseRepository>(MockBehavior.Strict);
        installedLicenseRepository
            .Setup(x => x.GetAsync(It.IsAny<CancellationToken>()))
            .ReturnsAsync(installed);
        var unitOfWork = new Mock<IUnitOfWork>(MockBehavior.Strict);
        unitOfWork.SetupGet(x => x.InstanceIdentity).Returns(identityRepository.Object);
        unitOfWork.SetupGet(x => x.InstalledLicense).Returns(installedLicenseRepository.Object);
        unitOfWork
            .Setup(x => x.CommitAsync(It.IsAny<CancellationToken>()))
            .Returns(Task.CompletedTask);
        unitOfWork
            .Setup(x => x.DisposeAsync())
            .Returns(ValueTask.CompletedTask);

        var verifier = new Mock<ILicenseVerifier>(MockBehavior.Strict);
        verifier
            .Setup(x => x.Verify(installed.RawLicense, identity, now))
            .Returns(LicenseVerificationResult.Succeeded(verifiedLicense));

        await using var services = new ServiceCollection()
            .AddSingleton(unitOfWork.Object)
            .BuildServiceProvider();
        var provider = new LicenseStateProvider(
            services.GetRequiredService<IServiceScopeFactory>(),
            timeProvider,
            verifier.Object);

        var initial = await provider.GetCurrentAsync(CancellationToken.None);
        timeProvider.Advance(TimeSpan.FromMinutes(10));
        var afterExpiry = await provider.GetCurrentAsync(CancellationToken.None);

        Assert.Equal(LicenseStatus.Valid, initial.Status);
        Assert.Equal(LicenseConstants.EditionTeam, initial.EffectiveEdition);
        Assert.Equal(LicenseStatus.Expired, afterExpiry.Status);
        Assert.Equal(LicenseConstants.EditionCommunity, afterExpiry.EffectiveEdition);
        verifier.Verify(x => x.Verify(installed.RawLicense, identity, now), Times.Once);
        installedLicenseRepository.Verify(
            x => x.GetAsync(It.IsAny<CancellationToken>()),
            Times.Once);
    }

    private sealed class MutableTimeProvider(DateTimeOffset now) : TimeProvider
    {
        private DateTimeOffset utcNow = now;

        public override DateTimeOffset GetUtcNow() => utcNow;

        public void Advance(TimeSpan duration) => utcNow += duration;
    }
}
