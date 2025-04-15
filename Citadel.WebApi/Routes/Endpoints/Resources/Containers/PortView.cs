namespace WebApi.Routes.Endpoints.Resources.Containers;

public record struct PortView(
    string IP,
    ushort PrivatePort,
    ushort PublicPort);