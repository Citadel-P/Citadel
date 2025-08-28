namespace Domain.Contracts.Resources.Images;

public sealed record RunImageInfoResult(
    IEnumerable<string> Volumes,
    IEnumerable<string> Networks,
    IEnumerable<string> ExposedPorts,
    double MemTotal,
    double CpuCount);