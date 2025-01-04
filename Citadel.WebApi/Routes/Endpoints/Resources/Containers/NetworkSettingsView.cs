using WebApi.Routes.Endpoints.Resources.Platforms;

namespace WebApi.Routes.Endpoints.Resources.Containers;

public sealed record NetworkSettingsView(IDictionary<string, EndpointSettingsView> Networks);