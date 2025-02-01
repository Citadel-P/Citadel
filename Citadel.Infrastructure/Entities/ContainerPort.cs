namespace Infrastructure.Entities;

public sealed record ContainerPort(string IP, int? PrivatePort, int? PublicPort);
