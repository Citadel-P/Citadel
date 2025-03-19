using System.Text.Json.Serialization;
using Refit;

namespace Infrastructure.GithubCr
{
    /// <summary>
    /// <see cref="https://docs.github.com/en/rest/packages/packages?apiVersion=2022-11-28"/>
    /// </summary>
    public partial interface IGithubCrApi
    {
        /// <summary>
        /// List container packages for a user
        /// </summary>
        /// <param name="username"></param>
        /// <param name="pat">Personal access token</param>
        /// <param name="agent">User-agent</param>
        /// <param name="cancellationToken"></param>
        /// <returns></returns>
        [Headers("Accept: application/vnd.github+json", "X-GitHub-Api-Version: 2022-11-28")]
        [Get("/users/{username}/packages?package_type=container&page=1&per_page=100")]
        Task<IEnumerable<GhcrPackage>> ListUserPackages(string username, [Authorize("Bearer")] string pat, [Header("User-Agent")] string agent, CancellationToken cancellationToken = default);

        /// <summary>
        /// List container packages for an organization
        /// </summary>
        /// <param name="org">organization name</param>
        /// <param name="pat">Personal access token</param>
        /// <param name="agent">user agent</param>
        /// <param name="cancellationToken"></param>
        /// <returns></returns>
        [Headers("Accept: application/vnd.github+json", "X-GitHub-Api-Version: 2022-11-28")]
        [Get("/orgs/{org}/packages?package_type=container&page=1&per_page=100")]
        Task<IEnumerable<GhcrPackage>> ListOrgPackages(string org, [Authorize("Bearer")] string pat, [Header("User-Agent")] string agent, CancellationToken cancellationToken = default);

        /// <summary>
        /// Lists package versions for a package owned by the authenticated user.
        /// </summary>
        /// <param name="package_type"></param>
        /// <param name="package_name"></param>
        /// <param name="pat">Personal access token</param>
        /// <param name="agent">user agent</param>
        /// <param name="cancellationToken"></param>
        /// <returns></returns>
        [Headers("Accept: application/vnd.github+json", "X-GitHub-Api-Version: 2022-11-28")]
        [Get("/user/packages/container/{package_name}/versions")]
        Task<IEnumerable<GhcrPackageVersion>> ListPackageVersionsForUser(string package_name, [Authorize("Bearer")] string pat, [Header("User-Agent")] string agent, int page = 1, int per_page = 100, CancellationToken cancellationToken = default);

        /// <summary>
        /// Lists package versions for a package owned by an organization.
        /// </summary>
        /// <param name="org"></param>
        /// <param name="pat"></param>
        /// <param name="agent"></param>
        /// <param name="cancellationToken"></param>
        /// <returns></returns>
        [Headers("Accept: application/vnd.github+json", "X-GitHub-Api-Version: 2022-11-28")]
        [Get("/orgs/{org}/packages/container/{package_name}/versions")]
        Task<IEnumerable<GhcrPackageVersion>> ListPackageVersionsForOrg(string org, string package_name, [Authorize("Bearer")] string pat, [Header("User-Agent")] string agent, int page = 1, int per_page = 100, CancellationToken cancellationToken = default);

    }
    public class GhcrPackage
    {
        public int Id { get; init; }
        public string Name { get; init; }
        public string Url { get; init; }
        [JsonPropertyName("package_type")]
        public string PackageType { get; init; }
        [JsonPropertyName("version_count")]
        public string VersionCount { get; init; }
        [JsonPropertyName("created_at")]
        public string CreatedAt { get; init; }
        [JsonPropertyName("updated_at")]
        public string UpdatedAt { get; init; }
        [JsonPropertyName("html_url")]
        public string HtmlUrl { get; init; }
        public Owner Owner { get; init; }
    }

    public class Owner
    {
        public int Id { get; init; }
        public string Url { get; init; }
        [JsonPropertyName("html_url")]
        public string HmlUrl { get; init; }
        [JsonPropertyName("node_id")]
        public string NodeId { get; init; }
        [JsonPropertyName("gists_url")]
        public string GistsUrl { get; init; }
    }

    public record GhcrPackageVersion
    {
        public int Id { get; init; }
        public string Name { get; init; }
        public string Url { get; init; }
        [JsonPropertyName("package_html_url")]
        public string PackageHtmlUrl { get; init; }
        [JsonPropertyName("created_at")]
        public string CreatedAt { get; init; }
        [JsonPropertyName("updated_at")]
        public string UpdatedAt { get; init; }
        [JsonPropertyName("html_url")]
        public string HtmlUrl { get; init; }
        [JsonPropertyName("metadata")]
        public PackageVersionMetadata Metadata { get; init; }
    }

    public record PackageVersionMetadata
    {
        [JsonPropertyName("container")]
        public PackageVersionContainerMetadata Container {  get; init; }
    }
    public record PackageVersionContainerMetadata
    {
        [JsonPropertyName("tags")]
        public IEnumerable<string> Tags { get; init; }

    }
}
