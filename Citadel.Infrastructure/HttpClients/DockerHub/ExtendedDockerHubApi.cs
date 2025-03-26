using System.Text.Json.Serialization;
using Refit;

namespace Infrastructure.DockerHub;

public partial interface IDockerHubApi
{
    [Get("/v2/repositories/{username}")]
    Task<PaginateRepositories> GetRepositories(string username, [Header("Authorization")] string accessToken, [Query] int? page, [Query] int? page_size, CancellationToken cancellationToken = default);

}

[System.CodeDom.Compiler.GeneratedCode("NJsonSchema", "14.2.0.0 (NJsonSchema v11.1.0.0 (Newtonsoft.Json v13.0.0.0))")]
public partial class PaginateRepositories : Page
{

    [JsonPropertyName("results")]
    public ICollection<DockerHubRepository> Results { get; set; }

}

[System.CodeDom.Compiler.GeneratedCode("NJsonSchema", "14.2.0.0 (NJsonSchema v11.1.0.0 (Newtonsoft.Json v13.0.0.0))")]
public partial class DockerHubRepository
{
    /// <summary>
    /// The name of the repository.
    /// </summary>
    [JsonPropertyName("name")]
    public string Name { get; set; }
    /// <summary>
    /// The namespace under which the repository exists.
    /// </summary>
    [JsonPropertyName("namespace")]
    public string Namespace { get; set; }
    /// <summary>
    /// Timestamp of the last update to the repository.
    /// </summary
    [JsonPropertyName("last_updated")]
    public DateTime LastUpdated { get; set; }
    /// <summary>
    /// Boolean indicating whether the repository is private
    /// </summary>
    [JsonPropertyName("is_private")]
    public bool IsPrivate { get; set; }
    /// <summary>
    /// Boolean indicating whether the repository is trusted.
    /// </summary>
    [JsonPropertyName("is_trusted")]
    public bool IsTrusted { get; set; }
    /// <summary>
    /// Boolean indicating whether the repository uses automated builds.
    /// </summary>
    [JsonPropertyName("is_automated")]
    public bool IsAutomated { get; set; }
    /// <summary>
    /// Total number of times the repository has been pulled (downloaded).
    /// </summary>
    [JsonPropertyName("pull_count")]
    public int PullCount { get; set; }
}