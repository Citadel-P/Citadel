namespace WebApi.Routes.Endpoints.Resources;

public sealed record ApplicationInfoView(
    string Name,
    string Version,
    string InformationalVersion);
