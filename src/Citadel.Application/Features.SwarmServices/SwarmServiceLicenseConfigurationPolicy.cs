using Domain;
using Domain.Entities.SwarmServices;

namespace Application.Features.SwarmServices;

internal static class SwarmServiceLicenseConfigurationPolicy
{
    public static bool ExpandsOperationalGuardrails(
        SwarmServiceSpec? current,
        SwarmServiceSpec proposed) =>
        proposed.UpdateBehavior == UpdateBehavior.AutoDeploy
        && current?.UpdateBehavior != UpdateBehavior.AutoDeploy;

    public static bool ExpandsAutomatedOperations(
        SwarmServiceSpec? current,
        SwarmServiceSpec proposed) =>
        proposed.Webhook is { Enabled: true }
        && current?.Webhook != proposed.Webhook;
}
