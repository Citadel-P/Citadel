namespace Infrastructure.Entities;

public sealed record ContainerPort(string IP, ushort PrivatePort, ushort PublicPort);
