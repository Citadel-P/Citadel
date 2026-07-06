using Hosting.Common.MergePatch;
using System.Reflection;
using System.Text.Json;

namespace WebApi.Routes.Endpoints.Resources.Oidc;

public sealed class UpdateOidcProviderPatchDocument : JsonMergePatchDocument<UpdateOidcProviderInput>
{
    public bool ContainsProperty(string propertyName)
        => Patch.ValueKind == JsonValueKind.Object && Patch.TryGetProperty(propertyName, out _);

    public static async ValueTask<UpdateOidcProviderPatchDocument?> BindAsync(HttpContext context, ParameterInfo _)
    {
        using var doc = await JsonDocument.ParseAsync(context.Request.Body);
        return new UpdateOidcProviderPatchDocument { Patch = doc.RootElement.Clone() };
    }
}
