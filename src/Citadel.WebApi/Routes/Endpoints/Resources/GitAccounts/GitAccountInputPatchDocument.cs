using System.Reflection;
using System.Text.Json;
using Hosting.Common.MergePatch;

namespace WebApi.Routes.Endpoints.Resources.GitAccounts;

public sealed class GitAccountInputPatchDocument : JsonMergePatchDocument<GitAccountInput>
{
    public static async ValueTask<GitAccountInputPatchDocument?> BindAsync(HttpContext context, ParameterInfo _)
    {
        using var doc = await JsonDocument.ParseAsync(context.Request.Body);
        return new GitAccountInputPatchDocument { Patch = doc.RootElement.Clone() };
    }
}
