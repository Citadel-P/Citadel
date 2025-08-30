using Hosting.Common;
using Hosting.OpenApi;
using Domain;
using WebApi.Routes.Endpoints;
using WebApi.Routes.Endpoints.Resources;
using WebApi.Routes.Endpoints.Resources.Platforms;
using WebApi.Routes.Endpoints.Resources.Registries;

namespace WebApi.Routes;

public static class PublicEndpoints
{
    const string ImagesName = nameof(Images);
    const string ComposeName = nameof(Compose);
    const string VolumesName = nameof(Volumes);
    const string NetworksName = nameof(Networks);
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
                MapAuthEndpoints(auth);
            }
            var containers = group.MapGroup("/containers").WithTags(ContainersName).RequireAuthorization();
            {
                MapContainerEndpoints(containers);
            }
            var platforms = group.MapGroup("/platforms").WithTags(PlatformsName).RequireAuthorization();
            {
                MapPlatformEndpoints(platforms);
            }
            var registries = group.MapGroup("/registries").WithTags(RegistriesName).RequireAuthorization();
            {
                MapRegistryEndpoints(registries);
            }
            var images = group.MapGroup("/images").WithTags(ImagesName).RequireAuthorization();
            {
                MapImageEndpoints(images);
            }
            var networks = group.MapGroup("/networks").WithTags(NetworksName).RequireAuthorization();
            {
                MapNetworkEndpoints(networks);
            }
            var volumes = group.MapGroup("/volumes").WithTags(VolumesName).RequireAuthorization();
            {
                MapVolumeEndpoints(volumes);
            }
            var compose = group.MapGroup("/compose").WithTags(ComposeName).RequireAuthorization();
            {
                MapComposeEndpoints(compose);
            }
        }

        group.ProducesProblem(StatusCodes.Status500InternalServerError);
    }

    private static void MapAuthEndpoints(RouteGroupBuilder auth)
    {
        auth.MapGet("/refresh", Authentication.RefreshToken)
            .WithSummary("Request a new access token")
            .WithCookie(Constants.RefreshToken, "Refresh Token", true)
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status404NotFound)
            .WithName(AuthenticationName + "_" + nameof(Authentication.RefreshToken));

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
    }

    private static void MapContainerEndpoints(RouteGroupBuilder containers)
    {
        containers.MapGet("/{id}", Containers.GetById)
            .WithSummary("Get container by Id")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName(ContainersName + "_" + nameof(Containers.GetById));

        containers.MapGet("/{id}/info", Containers.GetInfo)
            .WithSummary("Get basic container details")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName(ContainersName + "_" + nameof(Containers.GetInfo));

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

        containers.MapPost("/", Containers.Create)
            .WithSummary("Create a container")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName(ContainersName + "_" + nameof(Containers.Create));

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

        containers.MapDelete("/", Containers.DeleteContainers)
            .WithSummary("Delete the given container(s)")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName(ContainersName + "_" + nameof(Containers.DeleteContainers));
    }

    private static void MapPlatformEndpoints(RouteGroupBuilder platforms)
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

        platforms.MapGet("{id}/containers", Platforms.ListContainers)
            .WithSummary("Returns the list of containers of the given platform")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName(PlatformsName + "_" + nameof(Platforms.ListContainers));

        platforms.MapPatch("/{id}", Platforms.Patch)
            .WithSummary("Patch a platform")
            .Accepts<PlatformInput>("application/merge-patch+json", "application/json")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status409Conflict)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName(PlatformsName + "_" + nameof(Platforms.Patch));

        platforms.MapPost("/", Platforms.Create)
            .WithSummary("Create a platform")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName(PlatformsName + "_" + nameof(Platforms.Create));

        platforms.MapDelete("/", Platforms.Delete)
            .WithSummary("Delete a platform")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName(PlatformsName + "_" + nameof(Platforms.Delete));

    }

    private static void MapRegistryEndpoints(RouteGroupBuilder registries)
    {
        registries.MapGet("/all", Registries.GetAll)
            .WithSummary("Get all registries")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName(RegistriesName + "_" + nameof(Registries.GetAll));

        registries.MapGet("/{id}", Registries.GetById)
            .WithSummary("Get registry by ID")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName(RegistriesName + "_" + nameof(Registries.GetById));

        registries.MapGet("/{id}/_cfg", Registries.GetWithConfig)
            .WithSummary("Get registry and it's configuration")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName(RegistriesName + "_" + nameof(Registries.GetWithConfig));

        registries.MapPost("/", Registries.Create)
            .WithSummary("Create a registry")
            .WithDescription($"A discriminator should be provided in the request, this discriminator is based on {nameof(RegistryType)} enum ")
            .WithExample(RegistryType.Azure.ToString(), Examples.Registries.Create.CreateAzureRegistryExample())
            .WithExample(RegistryType.AWS.ToString(), Examples.Registries.Create.CreateAwsRegistryExample())
            .WithExample(RegistryType.Gitlab.ToString(), Examples.Registries.Create.CreateGitlabRegistryExample())
            .WithExample(RegistryType.DockerHub.ToString(), Examples.Registries.Create.CreateDockerHubRegistryExample())
            .WithExample(RegistryType.GitHub.ToString(), Examples.Registries.Create.CreateGitHubRegistryExample())
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName(RegistriesName + "_" + nameof(Registries.Create));

        registries.MapPatch("/{id}", Registries.Patch)
            .WithSummary("Patch a registry")
            .WithDescription($"A discriminator should be provided in the request, this discriminator is based on {nameof(RegistryType)} enum")
            .Accepts<RegistryInput>("application/merge-patch+json", "application/json")
            .WithExample(RegistryType.Azure.ToString(), Examples.Registries.Update.UpdateAzureRegistryExample())
            .WithExample(RegistryType.AWS.ToString(), Examples.Registries.Update.UpdateAwsRegistryExample())
            .WithExample(RegistryType.Gitlab.ToString(), Examples.Registries.Update.UpdateGitlabRegistryExample())
            .WithExample(RegistryType.DockerHub.ToString(), Examples.Registries.Update.UpdateDockerHubRegistryExample())
            .WithExample(RegistryType.GitHub.ToString(), Examples.Registries.Update.UpdateGitHubRegistryExample())
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status409Conflict)

            .WithName(RegistriesName + "_" + nameof(Registries.Patch));

        registries.MapDelete("/", Registries.Delete)
            .WithSummary("Delete registries")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName(RegistriesName + "_" + nameof(Registries.Delete));
    }

    private static void MapImageEndpoints(RouteGroupBuilder images)
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

        images.MapGet("/dockerhub", Images.GetDockerHubPublicImages)
            .WithSummary("Search for DockerHub public images. If the image name is empty, a default list of Docker images will be returned.")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName(ImagesName + "_" + nameof(Images.GetDockerHubPublicImages));

        images.MapGet("{platformId}/{imageId}", Images.Inspect)
           .WithSummary("Inspect an image")
           .ProducesValidationProblem()
           .ProducesProblem(StatusCodes.Status403Forbidden)
           .ProducesProblem(StatusCodes.Status401Unauthorized)
           .ProducesProblem(StatusCodes.Status404NotFound)
           .WithName(ImagesName + "_" + nameof(Images.Inspect));

        images.MapGet("{platformId}/{imageId}/_info", Images.GetImageInfo)
           .WithSummary("Get image info")
           .ProducesValidationProblem()
           .ProducesProblem(StatusCodes.Status403Forbidden)
           .ProducesProblem(StatusCodes.Status401Unauthorized)
           .ProducesProblem(StatusCodes.Status404NotFound)
           .WithName(ImagesName + "_" + nameof(Images.GetImageInfo));

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

    private static void MapNetworkEndpoints(RouteGroupBuilder networks)
    {
        networks.MapGet("{id}", Networks.List)
           .WithSummary("List all networks")
           .ProducesValidationProblem()
           .ProducesProblem(StatusCodes.Status403Forbidden)
           .ProducesProblem(StatusCodes.Status401Unauthorized)
           .ProducesProblem(StatusCodes.Status409Conflict)
           .WithName(NetworksName + "_" + nameof(Networks.List));

        networks.MapGet("{platformId}/{networkId}", Networks.Inspect)
           .WithSummary("Inspect a network")
           .ProducesValidationProblem()
           .ProducesProblem(StatusCodes.Status403Forbidden)
           .ProducesProblem(StatusCodes.Status401Unauthorized)
           .ProducesProblem(StatusCodes.Status409Conflict)
           .WithName(NetworksName + "_" + nameof(Networks.Inspect));

        networks.MapPost("/", Networks.Create)
            .WithSummary("Create a network")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status409Conflict)
            .WithName(NetworksName + "_" + nameof(Networks.Create));

        networks.MapDelete("/", Networks.Delete)
            .WithSummary("Delete a network(s)")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status404NotFound)
            .WithName(NetworksName + "_" + nameof(Networks.Delete));
    }

    private static void MapVolumeEndpoints(RouteGroupBuilder volumes)
    {
        volumes.MapGet("{id}", Volumes.List)
           .WithSummary("List all volumes")
           .ProducesValidationProblem()
           .ProducesProblem(StatusCodes.Status403Forbidden)
           .ProducesProblem(StatusCodes.Status401Unauthorized)
           .ProducesProblem(StatusCodes.Status409Conflict)
           .WithName(VolumesName + "_" + nameof(Volumes.List));

        volumes.MapGet("{platformId}/{name}", Volumes.Inspect)
           .WithSummary("Inspect a volume")
           .ProducesValidationProblem()
           .ProducesProblem(StatusCodes.Status403Forbidden)
           .ProducesProblem(StatusCodes.Status401Unauthorized)
           .ProducesProblem(StatusCodes.Status409Conflict)
           .WithName(VolumesName + "_" + nameof(Volumes.Inspect));

        volumes.MapDelete("/", Volumes.Delete)
            .WithSummary("Delete a volume(s)")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status404NotFound)
            .WithName(VolumesName + "_" + nameof(Volumes.Delete));

        volumes.MapPost("/", Volumes.Create)
            .WithSummary("Create a volume")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status409Conflict)
            .WithName(VolumesName + "_" + nameof(Volumes.Create));
    }

    private static void MapComposeEndpoints(RouteGroupBuilder compose)
    {
        compose.MapPost("/up", Compose.Up)
            .WithSummary("Deploy a stack")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status404NotFound)
            .WithName(ComposeName + "_" + nameof(Compose.Up));
    }
}
