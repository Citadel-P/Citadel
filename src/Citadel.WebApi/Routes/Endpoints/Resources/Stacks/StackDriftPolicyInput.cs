using Domain;
using Domain.Entities.Stacks;

namespace WebApi.Routes.Endpoints.Resources.Stacks;

public sealed record StackDriftPolicyInput(
    StackDriftMode? Mode = null,
    bool? AlertOnDrift = null,
    bool? MarkDegraded = null,
    bool? AutoStartStoppedContainers = null,
    bool? AutoResumePausedContainers = null,
    bool? RemoveExtraContainers = null)
{
    internal StackDriftPolicy ToPolicy()
    {
        var defaults = StackDriftPolicy.Default;
        return new StackDriftPolicy(
            Mode: Mode ?? defaults.Mode,
            AlertOnDrift: AlertOnDrift ?? defaults.AlertOnDrift,
            MarkDegraded: MarkDegraded ?? defaults.MarkDegraded,
            AutoStartStoppedContainers: AutoStartStoppedContainers ?? defaults.AutoStartStoppedContainers,
            AutoResumePausedContainers: AutoResumePausedContainers ?? defaults.AutoResumePausedContainers,
            RemoveExtraContainers: RemoveExtraContainers ?? defaults.RemoveExtraContainers).Normalize();
    }
}
