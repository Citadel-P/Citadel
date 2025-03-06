using System;
using System.Collections.Generic;
using System.Linq;
using System.Text;
using System.Text.Json.Serialization;
using System.Threading.Tasks;
using Infrastructure.DockerHub;
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
    }
    public class GhcrPackage
    {
        public int Id { get; set; }
        public string Name { get; set; }
        public string Url { get; set; }
        [JsonPropertyName("package_type")]
        public string PackageType { get; set; }
        [JsonPropertyName("version_count")]
        public string VersionCount { get; set; }
        [JsonPropertyName("created_at")]
        public string CreatedAt { get; set; }
        [JsonPropertyName("updated_at")]
        public string UpdatedAt { get; set; }
        public Owner Owner { get; set; }
    }

    public class Owner
    {
        public int Id { get; set; }
        public string Url { get; set; }
        [JsonPropertyName("html_url")]
        public string HmlUrl { get; set; }
    }
}
