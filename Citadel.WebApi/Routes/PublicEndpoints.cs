using Hosting.Common;
using Hosting.OpenApi;
using Infrastructure;
using WebApi.Routes.Endpoints;
using WebApi.Routes.Endpoints.Resources;

namespace WebApi.Routes;

public static class PublicEndpoints
{
    const string ImagesName = nameof(Images);
    const string PlatformsName = nameof(Platforms);
    const string ContainersName = nameof(Containers);
    const string RegistriesName = nameof(Registries);
    const string AuthenticationName = nameof(Authentication);

    public static void MapPublicEndpoints(this WebApplication app)
    {
        var group = app.MapGroup("/api/v1").WithGroupName("v1");
        {
            var auth = group.MapGroup("/authentication").WithTags(AuthenticationName);
            {
                auth.MapPost("/login", Authentication.Login)
                     .WithSummary("Check user credentials and issue an access token on successful login")
                     .ProduceCookie(Constants.RefreshToken)
                     .ProducesValidationProblem()
                     .ProducesProblem(StatusCodes.Status404NotFound)
                     .WithName(AuthenticationName + "_" + nameof(Authentication.Login));

                auth.MapPost("/logout", Authentication.Logout)
                     .WithSummary("Log out")
                     .WithCookie(Constants.RefreshToken, "Refresh Token", true)
                     .ProducesValidationProblem()
                     .ProducesProblem(StatusCodes.Status403Forbidden)
                     .ProducesProblem(StatusCodes.Status401Unauthorized)
                     .ProducesProblem(StatusCodes.Status404NotFound)
                     .WithName(AuthenticationName + "_" + nameof(Authentication.Logout));

                auth.MapGet("/refresh", Authentication.RefreshToken)
                     .WithSummary("Request a new access token")
                     .WithCookie(Constants.RefreshToken, "Refresh Token", true)
                     .ProducesValidationProblem()
                     .ProducesProblem(StatusCodes.Status404NotFound)
                     .WithName(AuthenticationName + "_" + nameof(Authentication.RefreshToken));
            }
            var containers = group.MapGroup("/containers").WithTags(ContainersName).RequireAuthorization();
            {
                containers.MapGet("/{id}", Containers.GetById)
                     .WithSummary("Get container by Id")
                     .ProducesValidationProblem()
                     .ProducesProblem(StatusCodes.Status404NotFound)
                     .ProducesProblem(StatusCodes.Status403Forbidden)
                     .ProducesProblem(StatusCodes.Status401Unauthorized)
                     .WithName(ContainersName + "_" + nameof(Containers.GetById));

                containers.MapPatch("start", Containers.StartContainers)
                     .WithSummary("Starts the given container(s)")
                     .ProducesValidationProblem()
                     .ProducesProblem(StatusCodes.Status404NotFound)
                     .ProducesProblem(StatusCodes.Status403Forbidden)
                     .ProducesProblem(StatusCodes.Status401Unauthorized)
                     .WithName(ContainersName + "_" + nameof(Containers.StartContainers));

                containers.MapPatch("stop", Containers.StopContainers)
                     .WithSummary("Stops the given container(s)")
                     .ProducesValidationProblem()
                     .ProducesProblem(StatusCodes.Status404NotFound)
                     .ProducesProblem(StatusCodes.Status403Forbidden)
                     .ProducesProblem(StatusCodes.Status401Unauthorized)
                     .WithName(ContainersName + "_" + nameof(Containers.StopContainers));

                containers.MapPatch("pause", Containers.PauseContainers)
                     .WithSummary("Pause the given container(s)")
                     .ProducesValidationProblem()
                     .ProducesProblem(StatusCodes.Status404NotFound)
                     .ProducesProblem(StatusCodes.Status403Forbidden)
                     .ProducesProblem(StatusCodes.Status401Unauthorized)
                     .WithName(ContainersName + "_" + nameof(Containers.PauseContainers));

                containers.MapPatch("restart", Containers.RestartContainers)
                     .WithSummary("Restarts the given container(s)")
                     .ProducesValidationProblem()
                     .ProducesProblem(StatusCodes.Status404NotFound)
                     .ProducesProblem(StatusCodes.Status403Forbidden)
                     .ProducesProblem(StatusCodes.Status401Unauthorized)
                     .WithName(ContainersName + "_" + nameof(Containers.RestartContainers));

                containers.MapPatch("unpause", Containers.UnpauseContainers)
                     .WithSummary("Unpause the given container(s)")
                     .ProducesValidationProblem()
                     .ProducesProblem(StatusCodes.Status404NotFound)
                     .ProducesProblem(StatusCodes.Status403Forbidden)
                     .ProducesProblem(StatusCodes.Status401Unauthorized)
                     .WithName(ContainersName + "_" + nameof(Containers.UnpauseContainers));

                containers.MapDelete("delete", Containers.DeleteContainers)
                     .WithSummary("Delete the given container(s)")
                     .ProducesValidationProblem()
                     .ProducesProblem(StatusCodes.Status404NotFound)
                     .ProducesProblem(StatusCodes.Status403Forbidden)
                     .ProducesProblem(StatusCodes.Status401Unauthorized)
                     .WithName(ContainersName + "_" + nameof(Containers.DeleteContainers));

                containers.MapPost("stream-logs", Containers.StreamLogs)
                     .WithSummary("Stream container logs")
                     .ProducesValidationProblem()
                     .ProducesProblem(StatusCodes.Status404NotFound)
                     .ProducesProblem(StatusCodes.Status403Forbidden)
                     .ProducesProblem(StatusCodes.Status401Unauthorized)
                     .WithName(ContainersName + "_" + nameof(Containers.StreamLogs));

                containers.MapGet("{id}/stats", Containers.GetStats)
                    .WithSummary("Get container stats")
                    .ProducesValidationProblem()
                    .ProducesProblem(StatusCodes.Status404NotFound)
                    .ProducesProblem(StatusCodes.Status403Forbidden)
                    .ProducesProblem(StatusCodes.Status401Unauthorized)
                    .WithName(ContainersName + "_" + nameof(Containers.GetStats));

                containers.MapGet("{id}/inspect", Containers.Inspect)
                   .WithSummary("Inspect a container")
                   .ProducesValidationProblem()
                   .ProducesProblem(StatusCodes.Status404NotFound)
                   .ProducesProblem(StatusCodes.Status403Forbidden)
                   .ProducesProblem(StatusCodes.Status401Unauthorized)
                   .WithName(ContainersName + "_" + nameof(Containers.Inspect));
            }
            var platforms = group.MapGroup("/platforms").WithTags(PlatformsName).RequireAuthorization();
            {
                platforms.MapGet("/", Platforms.List)
                    .WithSummary("List all platforms")
                    .ProducesValidationProblem()
                    .ProducesProblem(StatusCodes.Status404NotFound)
                    .ProducesProblem(StatusCodes.Status403Forbidden)
                    .ProducesProblem(StatusCodes.Status401Unauthorized)
                    .WithName(PlatformsName + "_" + nameof(Platforms.List));

                platforms.MapGet("/{id}", Platforms.GetById)
                    .WithSummary("Get platform by Id")
                    .ProducesValidationProblem()
                    .ProducesProblem(StatusCodes.Status404NotFound)
                    .ProducesProblem(StatusCodes.Status403Forbidden)
                    .ProducesProblem(StatusCodes.Status401Unauthorized)
                    .WithName(PlatformsName + "_" + nameof(Platforms.GetById));

                platforms.MapGet("/{id}/info", Platforms.GetInfo)
                    .WithSummary("Get platform by Id")
                    .ProducesValidationProblem()
                    .ProducesProblem(StatusCodes.Status404NotFound)
                    .ProducesProblem(StatusCodes.Status403Forbidden)
                    .ProducesProblem(StatusCodes.Status401Unauthorized)
                    .WithName(PlatformsName + "_" + nameof(Platforms.GetInfo));

                platforms.MapPut("/", Platforms.Put)
                    .WithSummary("Create or update a platform")
                    .ProducesValidationProblem()
                    .ProducesProblem(StatusCodes.Status404NotFound)
                    .ProducesProblem(StatusCodes.Status403Forbidden)
                    .ProducesProblem(StatusCodes.Status401Unauthorized)
                    .WithName(PlatformsName + "_" + nameof(Platforms.Put));

                platforms.MapDelete("/", Platforms.Delete)
                    .WithSummary("Delete a platform")
                    .ProducesValidationProblem()
                    .ProducesProblem(StatusCodes.Status404NotFound)
                    .ProducesProblem(StatusCodes.Status403Forbidden)
                    .ProducesProblem(StatusCodes.Status401Unauthorized)
                    .WithName(PlatformsName + "_" + nameof(Platforms.Delete));

                platforms.MapGet("{id}/containers", Platforms.ListContainers)
                    .WithSummary("Returns the list of containers of the given platform")
                    .ProducesValidationProblem()
                    .ProducesProblem(StatusCodes.Status404NotFound)
                    .ProducesProblem(StatusCodes.Status403Forbidden)
                    .ProducesProblem(StatusCodes.Status401Unauthorized)
                    .WithName(PlatformsName + "_" + nameof(Platforms.ListContainers));
            }
            var registries = group.MapGroup("/registries").WithTags(RegistriesName).RequireAuthorization();
            {
                registries.MapPost("/", Registries.Create)
                    .WithSummary("Create a registry")
                    .WithDescription($"A discriminator should be provided in the request, this discriminator is based on {nameof(RegistryDiscriminator)} enum ")
                    .WithExample(RegistryDiscriminator.Azure.ToString(), Examples.Registries.Create.CreateAzureRegistryExample())
                    .WithExample(RegistryDiscriminator.AWS.ToString(), Examples.Registries.Create.CreateAwsRegistryExample())
                    .WithExample(RegistryDiscriminator.Gitlab.ToString(), Examples.Registries.Create.CreateGitlabRegistryExample())
                    .WithExample(RegistryDiscriminator.DockerHub.ToString(), Examples.Registries.Create.CreateDockerHubRegistryExample())
                    .WithExample(RegistryDiscriminator.GitHub.ToString(), Examples.Registries.Create.CreateGitHubRegistryExample())
                    .ProducesValidationProblem()
                    .ProducesProblem(StatusCodes.Status404NotFound)
                    .ProducesProblem(StatusCodes.Status403Forbidden)
                    .ProducesProblem(StatusCodes.Status401Unauthorized)
                    .WithName(RegistriesName + "_" + nameof(Registries.Create));

                registries.MapGet("/all", Registries.GetAll)
                    .WithSummary("Get all registries")
                    .ProducesValidationProblem()
                    .ProducesProblem(StatusCodes.Status403Forbidden)
                    .ProducesProblem(StatusCodes.Status401Unauthorized)
                    .WithName(RegistriesName + "_" + nameof(Registries.GetAll));

                registries.MapGet("/{id}", Registries.GetById)
                   .WithSummary("Get all registries")
                   .ProducesValidationProblem()
                   .ProducesProblem(StatusCodes.Status403Forbidden)
                   .ProducesProblem(StatusCodes.Status401Unauthorized)
                   .WithName(RegistriesName + "_" + nameof(Registries.GetById));

                registries.MapDelete("/", Registries.Delete)
                    .WithSummary("Delete registries")
                    .ProducesValidationProblem()
                    .ProducesProblem(StatusCodes.Status403Forbidden)
                    .ProducesProblem(StatusCodes.Status401Unauthorized)
                    .WithName(RegistriesName + "_" + nameof(Registries.Delete));

                registries.MapPatch("/", Registries.Patch)
                    .WithSummary("Patch a registry")
                    .WithDescription($"A discriminator should be provided in the request, this discriminator is based on {nameof(RegistryDiscriminator)} enum")
                    .WithExample(RegistryDiscriminator.Azure.ToString(), Examples.Registries.Update.UpdateAzureRegistryExample())
                    .WithExample(RegistryDiscriminator.AWS.ToString(), Examples.Registries.Update.UpdateAwsRegistryExample())
                    .WithExample(RegistryDiscriminator.Gitlab.ToString(), Examples.Registries.Update.UpdateGitlabRegistryExample())
                    .WithExample(RegistryDiscriminator.DockerHub.ToString(), Examples.Registries.Update.UpdateDockerHubRegistryExample())
                    .WithExample(RegistryDiscriminator.GitHub.ToString(), Examples.Registries.Update.UpdateGitHubRegistryExample())
                    .ProducesValidationProblem()
                    .ProducesProblem(StatusCodes.Status403Forbidden)
                    .ProducesProblem(StatusCodes.Status401Unauthorized)
                    .WithName(RegistriesName + "_" + nameof(Registries.Patch));
            }

            var images = group.MapGroup("/images").WithTags(ImagesName).RequireAuthorization();
            {
                images.MapGet("/{platformId}/local-images", Images.GetAllLocalImages)
                    .WithSummary("Get all local images for the given platform")
                    .ProducesValidationProblem()
                    .ProducesProblem(StatusCodes.Status403Forbidden)
                    .ProducesProblem(StatusCodes.Status401Unauthorized)
                    .WithName(ImagesName + "_" + nameof(Images.GetAllLocalImages));

                images.MapGet("/{registryName}/repositories", Images.GetExternalRepositories)
                    .WithSummary("List external repositories of the given registry")
                    .ProducesValidationProblem()
                    .ProducesProblem(StatusCodes.Status403Forbidden)
                    .ProducesProblem(StatusCodes.Status401Unauthorized)
                    .WithName(ImagesName + "_" + nameof(Images.GetExternalRepositories));

                images.MapGet("/ghcr/{registryName}/{packageName}/versions", Images.GetGhcrPackageVersions)
                    .WithSummary("List versions of GHCR package")
                    .ProducesValidationProblem()
                    .ProducesProblem(StatusCodes.Status403Forbidden)
                    .ProducesProblem(StatusCodes.Status401Unauthorized)
                    .WithName(ImagesName + "_" + nameof(Images.GetGhcrPackageVersions));

                images.MapGet("/dockerhub/{registryName}/repositories", Images.GetDockerHubRepositories)
                    .WithSummary("List DockerHub repositories")
                    .ProducesValidationProblem()
                    .ProducesProblem(StatusCodes.Status403Forbidden)
                    .ProducesProblem(StatusCodes.Status401Unauthorized)
                    .WithName(ImagesName + "_" + nameof(Images.GetDockerHubRepositories));

                images.MapGet("/dockerhub/{registryName}/{repositoryName}/tags", Images.GetDockerHubRepositoryTags)
                    .WithSummary("List DockerHub repository tags")
                    .ProducesValidationProblem()
                    .ProducesProblem(StatusCodes.Status403Forbidden)
                    .ProducesProblem(StatusCodes.Status401Unauthorized)
                    .WithName(ImagesName + "_" + nameof(Images.GetDockerHubRepositoryTags));

                images.MapPost("/pull", Images.PullImage)
                    .WithSummary("Pull an image from a registry and returns logs as a stream")
                    .ProducesValidationProblem()
                    .ProducesProblem(StatusCodes.Status403Forbidden)
                    .ProducesProblem(StatusCodes.Status401Unauthorized)
                    .WithName(ImagesName + "_" + nameof(Images.PullImage));

                images.MapDelete("/", Images.Delete)
                    .WithSummary("Remove an image(s), along with any untagged parent images that were referenced by that image")
                    .ProducesValidationProblem()
                    .ProducesProblem(StatusCodes.Status404NotFound)
                    .ProducesProblem(StatusCodes.Status403Forbidden)
                    .ProducesProblem(StatusCodes.Status401Unauthorized)
                    .WithName(ImagesName + "_" + nameof(Images.Delete));
            }
        }

        group.ProducesProblem(StatusCodes.Status500InternalServerError);
    }
}