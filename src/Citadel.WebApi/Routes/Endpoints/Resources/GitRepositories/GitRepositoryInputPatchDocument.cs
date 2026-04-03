using System.Reflection;
using System.Text.Json;
using Hosting.Common.MergePatch;

namespace WebApi.Routes.Endpoints.Resources.GitRepositories;

public sealed class GitRepositoryInputPatchDocument : JsonMergePatchDocument<PatchGitRepositoryInput>
{
    public static async ValueTask<GitRepositoryInputPatchDocument?> BindAsync(HttpContext context, ParameterInfo _)
    {
        using var doc = await JsonDocument.ParseAsync(context.Request.Body);
        return new GitRepositoryInputPatchDocument { Patch = doc.RootElement.Clone() };
    }
}
