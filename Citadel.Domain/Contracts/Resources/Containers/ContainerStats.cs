using Domain.Entities;

namespace Domain.Contracts.Resources.Containers;

public sealed record ContainerStats(IReadOnlyDictionary<string, ContainerStat> Containers);
