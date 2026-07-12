using Hosting.Common.MergePatch;
using System.Reflection;
using System.Text.Json;

namespace WebApi.Routes.Endpoints.Resources.Backups;

public sealed class UpdateBackupRepositoryInputPatchDocument : JsonMergePatchDocument<UpdateBackupRepositoryInput>
{
    public bool ContainsProperty(string propertyName)
        => Patch.ValueKind == JsonValueKind.Object && Patch.TryGetProperty(propertyName, out _);

    public static async ValueTask<UpdateBackupRepositoryInputPatchDocument?> BindAsync(HttpContext context, ParameterInfo _)
    {
        using var doc = await JsonDocument.ParseAsync(context.Request.Body);
        return new UpdateBackupRepositoryInputPatchDocument { Patch = doc.RootElement.Clone() };
    }
}

public sealed class UpdateBackupPolicyInputPatchDocument : JsonMergePatchDocument<UpdateBackupPolicyInput>
{
    public bool ContainsProperty(string propertyName)
        => Patch.ValueKind == JsonValueKind.Object && Patch.TryGetProperty(propertyName, out _);

    public static async ValueTask<UpdateBackupPolicyInputPatchDocument?> BindAsync(HttpContext context, ParameterInfo _)
    {
        using var doc = await JsonDocument.ParseAsync(context.Request.Body);
        return new UpdateBackupPolicyInputPatchDocument { Patch = doc.RootElement.Clone() };
    }
}
