using System.Text.Json;
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
    public static readonly JsonSerializerOptions JsonSerializerOptions = JsonOptions();
    private static JsonSerializerOptions JsonOptions()
    {
        var serializerOptions = new JsonSerializerOptions()
        {
            PropertyNamingPolicy = null,
            PropertyNameCaseInsensitive = false,
            DefaultIgnoreCondition = JsonIgnoreCondition.WhenWritingNull,
            NumberHandling = JsonNumberHandling.AllowNamedFloatingPointLiterals
        };

        // Resolvers
        serializerOptions.TypeInfoResolverChain.Add(DockerHubContext.Default);
        serializerOptions.TypeInfoResolverChain.Add(GitHubCrContext.Default);

        // Converters
        serializerOptions.Converters.Add(new JsonStringEnumConverter());

        return serializerOptions;
    }
}
