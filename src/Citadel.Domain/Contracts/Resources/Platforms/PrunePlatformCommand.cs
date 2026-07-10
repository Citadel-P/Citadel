using Domain;

namespace Domain.Contracts.Resources.Platforms;

public sealed record PrunePlatformCommand(string PlatformAddress, PruneResource Resource);
