namespace Application.Features.Platforms.Queries.Models;

public sealed record GetContainersQuery(
    bool? All,
    int? Limit = null,
    bool? Size = null,
    IDictionary<string, IDictionary<string, bool>>? Filters = null);