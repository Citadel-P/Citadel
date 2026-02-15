namespace Domain.Contracts.Resources.Images;

public sealed record DistributionInspectCommand(string PlatformAddress, string ImageName, string? Auth);

