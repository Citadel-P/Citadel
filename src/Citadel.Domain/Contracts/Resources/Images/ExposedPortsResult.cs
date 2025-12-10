namespace Domain.Contracts.Resources.Images;

public sealed record ExposedPortsResult(IEnumerable<string> Ports);