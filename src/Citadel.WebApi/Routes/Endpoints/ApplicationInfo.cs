using Microsoft.AspNetCore.Http.HttpResults;
using System.Reflection;
using WebApi.Routes.Endpoints.Resources;

namespace WebApi.Routes.Endpoints;

public static class ApplicationInfo
{
    public static Ok<ApplicationInfoView> Get()
    {
        var informationalVersion = GetInformationalVersion();
        return TypedResults.Ok(new ApplicationInfoView(
            "Citadel",
            GetDisplayVersion(informationalVersion),
            informationalVersion));
    }

    private static string GetInformationalVersion()
    {
        var assembly = typeof(ApplicationInfo).Assembly;
        return assembly.GetCustomAttribute<AssemblyInformationalVersionAttribute>()?.InformationalVersion
               ?? assembly.GetName().Version?.ToString()
               ?? "0.0.0";
    }

    private static string GetDisplayVersion(string informationalVersion)
    {
        var plusIndex = informationalVersion.IndexOf('+', StringComparison.Ordinal);
        return plusIndex > 0 ? informationalVersion[..plusIndex] : informationalVersion;
    }
}
