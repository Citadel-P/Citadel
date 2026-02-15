namespace Domain.Contracts.Resources.Images;

public sealed record DistributionResult(OCIDescriptorResult Descriptor);

public sealed record OCIDescriptorResult(string MediaType, string Digest, long? Size, OCIPlatformResult Platform, string ArtifactType);

public sealed record OCIPlatformResult(string Architecture, string Os, string OsVersion);
