using Domain;
using Domain.Contracts.Resources.Stacks;
using Domain.Entities.Stacks;
using System.Text.Json;

namespace Infrastructure.Persistence.Dtos;

internal sealed record StackDriftStackDto(
        Guid Id,
        Guid CurrentStackReleaseId,
        string Name,
        string StackSource,
        string ControlState,
        string? DriftPolicy,
        Guid PlatformId,
        string Status,
        string Spec,
        string? PlatformName)
{
    public StackDriftStack ToDriftStack()
        => new(
            Id,
            CurrentStackReleaseId,
            Name,
            Enum.Parse<StackSource>(StackSource),
            PlatformId,
            PlatformName,
            Enum.Parse<StackReleaseStatus>(Status),
            Enum.Parse<ResourceControlState>(ControlState),
            JsonSerializer.Deserialize(Spec, StackJsonContext.Default.StackSpec)!,
            string.IsNullOrWhiteSpace(DriftPolicy)
                ? StackDriftPolicy.Default
                : JsonSerializer.Deserialize(DriftPolicy, StackJsonContext.Default.StackDriftPolicy) ?? StackDriftPolicy.Default);
}
