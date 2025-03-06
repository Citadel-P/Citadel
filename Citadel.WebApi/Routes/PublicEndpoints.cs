using Application.Utils;
using Hosting.OpenApi;
using Infrastructure;
using WebApi.Routes.Endpoints;
using WebApi.Routes.Endpoints.Resources;

namespace WebApi.Routes;

public static class PublicEndpoints
{
    const string ContainersName = nameof(Containers);
    const string PlatformsName = nameof(Platforms);
    const string RegistriesName = nameof(Registries);
    const string AuthenticationName = nameof(Authentication);

    public static void MapPublicEndpoints(this WebApplication app)
    {
        var group = app.MapGroup("/api/v1").WithGroupName("v1"); ;
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

                containers.MapPatch("delete", Containers.DeleteContainers)
                     .WithSummary("Delete the given container(s)")
                     .ProducesValidationProblem()
                     .ProducesProblem(StatusCodes.Status404NotFound)
                     .ProducesProblem(StatusCodes.Status403Forbidden)
                     .ProducesProblem(StatusCodes.Status401Unauthorized)
                     .WithName(ContainersName + "_" + nameof(Containers.DeleteContainers));

                containers.MapPost("stream-logs", Containers.StreamLogs)
                     .WithSummary("Request to start (or stop) streaming container logs")
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
                    .WithExample(RegistryDiscriminator.Azure.ToString(), Examples.Registries.CreateAzureRegistryExample())
                    .WithExample(RegistryDiscriminator.AWS.ToString(), Examples.Registries.CreateAwsRegistryExample())
                    .WithExample(RegistryDiscriminator.Gitlab.ToString(), Examples.Registries.CreateGitlabRegistryExample())
                    .WithExample(RegistryDiscriminator.DockerHub.ToString(), Examples.Registries.CreateDockerHubRegistryExample())
                    .WithExample(RegistryDiscriminator.GitHub.ToString(), Examples.Registries.CreateGitHubRegistryExample())
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

                registries.MapDelete("/", Registries.Delete)
                    .WithSummary("Get all registries")
                    .ProducesValidationProblem()
                    .ProducesProblem(StatusCodes.Status403Forbidden)
                    .ProducesProblem(StatusCodes.Status401Unauthorized)
                    .WithName(RegistriesName + "_" + nameof(Registries.Delete));
            }
        }

        group.ProducesProblem(StatusCodes.Status500InternalServerError);
    }
}