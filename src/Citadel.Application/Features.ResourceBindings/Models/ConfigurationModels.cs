using Domain;
using Domain.Entities.ResourceBindings;

namespace Application.Features.ResourceBindings.Models;

public sealed record ResourceBindingInput(
    string Name,
    ResourceBindingKind Kind,
    string? Value,
    Guid? SecretId,
    SecretDeliveryMode? SecretDeliveryMode = null,
    string? TargetPath = null);

public sealed record ResourceBindingsResult(
    IReadOnlyList<ResourceBinding> Entries,
    IReadOnlyList<ResourceBinding> EffectiveEntries);

public sealed record CreateExternalSecretInputModel(
    string Name,
    Guid ProviderId,
    string ExternalPath,
    string ExternalKey,
    int? ExternalVersion);

public sealed record CreateVaultKvV2SecretProviderInputModel(
    string Name,
    string Address,
    string MountPath,
    string Token);
