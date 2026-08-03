using System.Reflection;
using System.Text.Json;
using Hosting.Common.MergePatch;

namespace WebApi.Routes.Endpoints.Resources.Deployments;

public class DeploymentInputPatchDocument : JsonMergePatchDocument<PatchDeploymentInput>
{
    private static readonly HashSet<string> AllowedProperties =
        new(StringComparer.OrdinalIgnoreCase) { "platformId", "spec" };

    public static async ValueTask<DeploymentInputPatchDocument?> BindAsync(HttpContext context, ParameterInfo _)
    {
        using var doc = await JsonDocument.ParseAsync(context.Request.Body);
        return new DeploymentInputPatchDocument { Patch = doc.RootElement.Clone() };
    }

    public bool HasOnlyConfigurationProperties()
        => Patch.ValueKind == JsonValueKind.Object
           && Patch.EnumerateObject().All(property => AllowedProperties.Contains(property.Name));
}
