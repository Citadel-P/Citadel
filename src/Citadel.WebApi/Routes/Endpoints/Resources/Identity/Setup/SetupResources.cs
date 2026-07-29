using Application.Features.Identity.Setup;

namespace WebApi.Routes.Endpoints.Resources.Identity.Setup;

public sealed record SetupStatusView(bool RequiresSetup)
{
    internal static SetupStatusView Map(SetupStatus status)
        => new(status.RequiresSetup);
}

public sealed record InitializeCitadelInput(
    string Name,
    string Email,
    string Password)
{
    internal InitializeCitadel ToCommand() => new(Name, Email, Password);
}
