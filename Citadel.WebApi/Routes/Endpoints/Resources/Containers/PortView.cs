namespace WebApi.Routes.Endpoints.Resources.Containers;

public sealed record PortView(string IP, ushort PrivatePort, ushort PublicPort);