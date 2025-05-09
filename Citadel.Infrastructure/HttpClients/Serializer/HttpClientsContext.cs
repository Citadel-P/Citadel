using System.Text.Json.Serialization;
using System.Text.Json.Serialization.Metadata;
using Infrastructure.DockerHub;
using Infrastructure.GithubCr;

namespace Infrastructure.HttpClients.Serializer;

[JsonSourceGenerationOptions(GenerationMode = JsonSourceGenerationMode.Default)]
[JsonSerializable(typeof(AuthCreateTokenResponse))]
[JsonSerializable(typeof(IDictionary<string, object>))]
[JsonSerializable(typeof(PaginateRepositories))]
[JsonSerializable(typeof(ICollection<DockerHubRepository>))]
[JsonSerializable(typeof(Page))]
[JsonSerializable(typeof(Body))]
[JsonSerializable(typeof(Paginated_tags))]
[JsonSerializable(typeof(ICollection<Tag>))]
[JsonSerializable(typeof(IList<Image>))]
[JsonSerializable(typeof(ICollection<Layer>))]
[JsonSerializable(typeof(PaginateImageSearch))]
[JsonSerializable(typeof(ICollection<DockerHubImageModel>))]
public partial class DockerHubContext : JsonSerializerContext
{

}

[JsonSourceGenerationOptions(GenerationMode = JsonSourceGenerationMode.Default)]
[JsonSerializable(typeof(IEnumerable<GhcrPackage>))]
[JsonSerializable(typeof(Owner))]
[JsonSerializable(typeof(GhcrPackageVersion))]
[JsonSerializable(typeof(PackageVersionMetadata))]
[JsonSerializable(typeof(PackageVersionContainerMetadata))]
[JsonSerializable(typeof(IEnumerable<GhcrPackageVersion>))]

public partial class GitHubCrContext : JsonSerializerContext
{ 
}

internal class HttpClientsContext
{
    public static void RegisterContexts(IList<IJsonTypeInfoResolver> typeInfoResolverChain)
    {
        typeInfoResolverChain.Add(DockerHubContext.Default);
        typeInfoResolverChain.Add(GitHubCrContext.Default);
    }
}
