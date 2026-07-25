using Application.Features.Stacks.Commands;
using Application.Services.Licensing;
using Domain;
using Domain.Entities.Licensing;
using Domain.Entities.Stacks;
using LightResults;

namespace Tests.Unit.Application.Features.Stacks;

public sealed class StackLicenseConfigurationPolicyTests
{
    [Fact]
    public async Task EnsureAllowedAsync_Should_Allow_Unrelated_Spec_Changes_To_Preserved_Paid_Configuration()
    {
        var current = ManualSpec(
            "services:\n  app:\n    image: example:v1",
            StackUpdateBehavior.StackAutoDeploy);
        var proposed = current with
        {
            ComposeFile = "services:\n  app:\n    image: example:v2"
        };
        var entitlements = new RecordingEntitlementService();

        var result = await StackLicenseConfigurationPolicy.EnsureAllowedAsync(
            current,
            StackDriftPolicy.Disabled,
            proposed,
            StackDriftPolicy.Disabled,
            entitlements,
            CancellationToken.None);

        Assert.True(result.IsSuccess());
        Assert.Empty(entitlements.Requested);
    }

    [Fact]
    public async Task EnsureAllowedAsync_Should_Allow_A_Guardrail_Reduction_After_Downgrade()
    {
        var spec = ManualSpec("services: {}", StackUpdateBehavior.Disabled);
        var current = new StackDriftPolicy(
            StackDriftMode.AutoFix,
            AlertOnDrift: true,
            MarkDegraded: true,
            AutoStartStoppedContainers: true,
            AutoResumePausedContainers: true,
            RemoveExtraContainers: true);
        var proposed = StackDriftPolicy.Default;
        var entitlements = new RecordingEntitlementService();

        var result = await StackLicenseConfigurationPolicy.EnsureAllowedAsync(
            spec,
            current,
            spec,
            proposed,
            entitlements,
            CancellationToken.None);

        Assert.True(result.IsSuccess());
        Assert.Empty(entitlements.Requested);
    }

    [Fact]
    public async Task EnsureAllowedAsync_Should_Require_Automated_Operations_For_A_New_Redeploy_Binding()
    {
        var buildProjectId = Guid.Parse("11111111-1111-1111-1111-111111111111");
        var current = ManualSpec("services: {}", StackUpdateBehavior.Disabled);
        var proposed = current with
        {
            BuildImageBindings =
            [
                new StackBuildImageBinding("app", buildProjectId, RedeployOnBuild: true)
            ]
        };
        var entitlements = new RecordingEntitlementService();

        var result = await StackLicenseConfigurationPolicy.EnsureAllowedAsync(
            current,
            StackDriftPolicy.Disabled,
            proposed,
            StackDriftPolicy.Disabled,
            entitlements,
            CancellationToken.None);

        Assert.True(result.IsSuccess());
        Assert.Equal([LicenseCapability.AutomatedOperations], entitlements.Requested);
    }

    [Fact]
    public async Task EnsureAllowedAsync_Should_Require_Guardrails_When_Drift_AutoFix_Is_Enabled()
    {
        var spec = ManualSpec("services: {}", StackUpdateBehavior.Disabled);
        var entitlements = new RecordingEntitlementService();

        var result = await StackLicenseConfigurationPolicy.EnsureAllowedAsync(
            spec,
            StackDriftPolicy.Disabled,
            spec,
            StackDriftPolicy.Default with { Mode = StackDriftMode.AutoFix },
            entitlements,
            CancellationToken.None);

        Assert.True(result.IsSuccess());
        Assert.Equal([LicenseCapability.OperationalGuardrails], entitlements.Requested);
    }

    private static ManualStack ManualSpec(
        string composeFile,
        StackUpdateBehavior updateBehavior)
        => new(
            ComposeFile: composeFile,
            UpdateBehavior: updateBehavior);

    private sealed class RecordingEntitlementService : ILicenseEntitlementService
    {
        public List<LicenseCapability> Requested { get; } = [];

        public ValueTask<LicenseState> GetOverviewAsync(CancellationToken cancellationToken)
            => throw new NotSupportedException();

        public ValueTask<bool> IsEnabledAsync(
            LicenseCapability capability,
            CancellationToken cancellationToken)
            => throw new NotSupportedException();

        public ValueTask<Result> EnsureEnabledAsync(
            LicenseCapability capability,
            CancellationToken cancellationToken)
        {
            Requested.Add(capability);
            return ValueTask.FromResult(Result.Success());
        }
    }
}
