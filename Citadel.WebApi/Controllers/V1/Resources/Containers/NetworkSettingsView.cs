using WebApi.Controllers.V1.Resources.Platforms;

namespace WebApi.Controllers.V1.Resources.Containers;

public sealed record NetworkSettingsView(IDictionary<string, EndpointSettingsView> Networks);