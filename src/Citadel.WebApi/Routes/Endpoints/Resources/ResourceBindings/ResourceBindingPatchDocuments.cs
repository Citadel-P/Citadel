using Hosting.Common.MergePatch;
using System.Reflection;
using System.Text.Json;

namespace WebApi.Routes.Endpoints.Resources.ResourceBindings;

public sealed class UpdateExternalSecretPatchDocument : JsonMergePatchDocument<UpdateExternalSecretInput>
{
    public bool ContainsProperty(string propertyName)
        => Patch.ValueKind == JsonValueKind.Object && Patch.TryGetProperty(propertyName, out _);

    public static async ValueTask<UpdateExternalSecretPatchDocument?> BindAsync(HttpContext context, ParameterInfo _)
    {
        using var doc = await JsonDocument.ParseAsync(context.Request.Body);
        return new UpdateExternalSecretPatchDocument { Patch = doc.RootElement.Clone() };
    }
}

public sealed class UpdateVaultKvV2SecretProviderPatchDocument : JsonMergePatchDocument<UpdateVaultKvV2SecretProviderInput>
{
    public static async ValueTask<UpdateVaultKvV2SecretProviderPatchDocument?> BindAsync(HttpContext context, ParameterInfo _)
    {
        using var doc = await JsonDocument.ParseAsync(context.Request.Body);
        return new UpdateVaultKvV2SecretProviderPatchDocument { Patch = doc.RootElement.Clone() };
    }
}
