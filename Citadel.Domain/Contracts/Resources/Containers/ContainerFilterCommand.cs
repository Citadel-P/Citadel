namespace Domain.Contracts.Resources.Containers;

public sealed record ContainerFilterCommand(
    string PlatformAddress,
    Guid PlatformId,
    bool? All = false,
    int? Limit = null,
    bool? Size = null,
    IDictionary<string, IDictionary<string, bool>>? Filters = null);
