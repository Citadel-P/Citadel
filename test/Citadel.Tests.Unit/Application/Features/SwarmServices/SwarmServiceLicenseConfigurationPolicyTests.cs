using Application.Features.SwarmServices;
using Domain;
using Domain.Entities.SwarmServices;

namespace Tests.Unit.Application.Features.SwarmServices;

public sealed class SwarmServiceLicenseConfigurationPolicyTests
{
    [Fact]
    public void ExpandsAutomatedOperations_ShouldDetectEnablingOrChangingAnActiveWebhook()
    {
        var disabled = CreateSpec(new SwarmServiceWebhookConfig(Enabled: false));
        var enabled = CreateSpec(CreateEnabledWebhook("first-secret"));
        var changed = CreateSpec(CreateEnabledWebhook("rotated-secret"));

        Assert.True(SwarmServiceLicenseConfigurationPolicy.ExpandsAutomatedOperations(disabled, enabled));
        Assert.True(SwarmServiceLicenseConfigurationPolicy.ExpandsAutomatedOperations(enabled, changed));
        Assert.False(SwarmServiceLicenseConfigurationPolicy.ExpandsAutomatedOperations(enabled, enabled));
    }

    [Fact]
    public void ExpandsAutomatedOperations_ShouldAllowDisablingAnActiveWebhook()
    {
        var enabled = CreateSpec(CreateEnabledWebhook("shared-secret"));
        var disabled = CreateSpec(enabled.Webhook! with { Enabled = false });

        Assert.False(SwarmServiceLicenseConfigurationPolicy.ExpandsAutomatedOperations(enabled, disabled));
    }

    private static SwarmServiceSpec CreateSpec(SwarmServiceWebhookConfig? webhook) => new()
    {
        Image = new SwarmExternalImage(Guid.CreateVersion7(), "redis:latest"),
        UpdateBehavior = UpdateBehavior.Notify,
        Webhook = webhook,
    };

    private static SwarmServiceWebhookConfig CreateEnabledWebhook(string secret) => new(
        Enabled: true,
        Provider: WebhookProvider.Generic,
        AuthScheme: WebhookAuthScheme.BearerToken,
        Secret: secret);
}
