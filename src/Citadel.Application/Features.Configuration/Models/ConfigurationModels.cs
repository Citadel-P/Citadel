using Domain.Entities.Configuration;

namespace Application.Features.Configuration.Models;

public sealed record ConfigurationEntryInput(
    string Name,
    ConfigurationEntryKind Kind,
    string? Value,
    Guid? SecretId,
    SecretDeliveryMode? SecretDeliveryMode = null,
    string? TargetPath = null);

public sealed record ConfigurationEntriesResult(
    IReadOnlyList<ConfigurationEntry> Entries,
    IReadOnlyList<ConfigurationEntry> EffectiveEntries);
