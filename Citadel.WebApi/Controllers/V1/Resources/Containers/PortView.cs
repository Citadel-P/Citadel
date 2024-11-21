namespace WebApi.Controllers.V1.Resources.Containers;

public sealed record PortView(string IP, ushort PrivatePort, ushort PublicPort);