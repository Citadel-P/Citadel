using Domain.Entities.Deployments;
using Domain.Entities.Stacks;

namespace Application.Services.Builds;

internal static class BuildImageProvenance
{
    public static DeploymentSpec Clear(DeploymentSpec spec)
        => spec.Image is BuildImage buildImage
            ? spec with { Image = buildImage.ClearProvenance() }
            : spec;

    public static DeploymentSpec Preserve(DeploymentSpec proposed, DeploymentSpec? current)
    {
        if (proposed.Image is not BuildImage proposedBuildImage)
            return proposed;

        return current?.Image is BuildImage currentBuildImage
            ? proposed with { Image = proposedBuildImage.PreserveProvenanceFrom(currentBuildImage) }
            : proposed with { Image = proposedBuildImage.ClearProvenance() };
    }

    public static StackSpec Clear(StackSpec spec)
        => spec.WithBuildImageBindings(spec.BuildImageBindings?
            .Select(static binding => binding.ClearProvenance())
            .ToArray());

    public static StackSpec Preserve(StackSpec proposed, StackSpec? current)
    {
        if (proposed.BuildImageBindings is not { Count: > 0 })
            return proposed;

        var currentBindings = current?.BuildImageBindings ?? [];
        var bindings = proposed.BuildImageBindings.Select(binding =>
        {
            var currentBinding = currentBindings.FirstOrDefault(candidate =>
                candidate.BuildProjectId == binding.BuildProjectId
                && string.Equals(candidate.ServiceName, binding.ServiceName, StringComparison.OrdinalIgnoreCase));

            return currentBinding is null
                ? binding.ClearProvenance()
                : binding.PreserveProvenanceFrom(currentBinding);
        }).ToArray();

        return proposed.WithBuildImageBindings(bindings);
    }
}
