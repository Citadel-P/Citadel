using Hosting.Common;
using Microsoft.AspNetCore.Http.HttpResults;
using WebApi.Routes.Endpoints.Resources;

namespace WebApi.Routes.Endpoints;

public static class ApplicationInfo
{
    public static Ok<ApplicationInfoView> Get()
    {
        var informationalVersion = Constants.CompatibilityVersion;
        return TypedResults.Ok(new ApplicationInfoView(
            "Citadel",
            GetDisplayVersion(informationalVersion),
            informationalVersion));
    }

    private static string GetDisplayVersion(string informationalVersion)
    {
        var plusIndex = informationalVersion.IndexOf('+', StringComparison.Ordinal);
        return plusIndex > 0 ? informationalVersion[..plusIndex] : informationalVersion;
    }
}
