using Hosting.Common.MergePatch;
using System.Reflection;
using System.Text.Json;

namespace WebApi.Routes.Endpoints.Resources;

public sealed record RenameResource(Guid Id, string Name);

public sealed record PatchResourceMetadata(string Description, string[] Tags);

public class PatchResourceMetadataDocument : JsonMergePatchDocument<PatchResourceMetadata>
{
    public static async ValueTask<PatchResourceMetadataDocument?> BindAsync(HttpContext context, ParameterInfo _)
    {
        using var doc = await JsonDocument.ParseAsync(context.Request.Body);
        return new PatchResourceMetadataDocument { Patch = doc.RootElement.Clone() };
    }
}
