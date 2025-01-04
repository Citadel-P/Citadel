using WebApi.Routes.Endpoints;
using WebApi.Routes.Endpoints.Resources.Containers;
using WebApi.Routes.Endpoints.Resources.Platforms;

namespace WebApi.Routes;

public static class PublicEndpoints
{
    const string ContainersName = nameof(Containers);
    const string PlatformsName = nameof(Platforms);
    const string AuthenticationName = nameof(Authentication);

    public static void MapPublicEndpoints(this WebApplication app)
    {
        var group = app.MapGroup("/api/v1").WithGroupName(Constants.PublicApiV1);
        {
            var auth = group.MapGroup("/authentication").WithTags(AuthenticationName);
            {
                auth.MapPost("/login", Authentication.Login)
                     .WithSummary("Check user credentials and issue a jwt token on successful login")
                     .Produces<ContainerInfoView>()
                     .ProducesValidationProblem()
                     .ProducesProblem(StatusCodes.Status404NotFound)
                     .ProducesProblem(StatusCodes.Status403Forbidden)
                     .ProducesProblem(StatusCodes.Status401Unauthorized)
                     .WithName(AuthenticationName + "_" + nameof(Authentication.Login));
            }
            var containers = group.MapGroup("/containers").WithTags(ContainersName).RequireAuthorization();
            {
                containers.MapGet("/{id}", Containers.GetById)
                     .WithSummary("Get container by Id")
                     .Produces<ContainerInfoView>()
                     .ProducesValidationProblem()
                     .ProducesProblem(StatusCodes.Status404NotFound)
                     .ProducesProblem(StatusCodes.Status403Forbidden)
                     .ProducesProblem(StatusCodes.Status401Unauthorized)
                     .WithName(ContainersName + "_" + nameof(Containers.GetById));

                containers.MapPatch("start", Containers.StartContainers)
                     .WithSummary("Starts the given container(s)")
                     .Produces(StatusCodes.Status204NoContent)
                     .ProducesValidationProblem()
                     .ProducesProblem(StatusCodes.Status404NotFound)
                     .ProducesProblem(StatusCodes.Status403Forbidden)
                     .ProducesProblem(StatusCodes.Status401Unauthorized)
                     .WithName(ContainersName + "_" + nameof(Containers.StartContainers));

                containers.MapPatch("stop", Containers.StopContainers)
                     .WithSummary("Stops the given container(s)")
                     .Produces(StatusCodes.Status204NoContent)
                     .ProducesValidationProblem()
                     .ProducesProblem(StatusCodes.Status404NotFound)
                     .ProducesProblem(StatusCodes.Status403Forbidden)
                     .ProducesProblem(StatusCodes.Status401Unauthorized)
                     .WithName(ContainersName + "_" + nameof(Containers.StopContainers));

                containers.MapPatch("pause", Containers.PauseContainers)
                     .WithSummary("Pause the given container(s)")
                     .Produces(StatusCodes.Status204NoContent)
                     .ProducesValidationProblem()
                     .ProducesProblem(StatusCodes.Status404NotFound)
                     .ProducesProblem(StatusCodes.Status403Forbidden)
                     .ProducesProblem(StatusCodes.Status401Unauthorized)
                     .WithName(ContainersName + "_" + nameof(Containers.PauseContainers));

                containers.MapPatch("restart", Containers.RestartContainers)
                     .WithSummary("Restarts the given container(s)")
                     .Produces(StatusCodes.Status204NoContent)
                     .ProducesValidationProblem()
                     .ProducesProblem(StatusCodes.Status404NotFound)
                     .ProducesProblem(StatusCodes.Status403Forbidden)
                     .ProducesProblem(StatusCodes.Status401Unauthorized)
                     .WithName(ContainersName + "_" + nameof(Containers.RestartContainers));

                containers.MapPatch("unpause", Containers.UnpauseContainers)
                     .WithSummary("Unpause the given container(s)")
                     .Produces(StatusCodes.Status204NoContent)
                     .ProducesValidationProblem()
                     .ProducesProblem(StatusCodes.Status404NotFound)
                     .ProducesProblem(StatusCodes.Status403Forbidden)
                     .ProducesProblem(StatusCodes.Status401Unauthorized)
                     .WithName(ContainersName + "_" + nameof(Containers.UnpauseContainers));

                containers.MapPatch("delete", Containers.DeleteContainers)
                     .WithSummary("Delete the given container(s)")
                     .Produces(StatusCodes.Status204NoContent)
                     .ProducesValidationProblem()
                     .ProducesProblem(StatusCodes.Status404NotFound)
                     .ProducesProblem(StatusCodes.Status403Forbidden)
                     .ProducesProblem(StatusCodes.Status401Unauthorized)
                     .WithName(ContainersName + "_" + nameof(Containers.DeleteContainers));

                containers.MapPost("stream-logs", Containers.StreamLogs)
                     .WithSummary("Request to start (or stop) streaming container logs")
                     .Produces(StatusCodes.Status204NoContent)
                     .ProducesValidationProblem()
                     .ProducesProblem(StatusCodes.Status404NotFound)
                     .ProducesProblem(StatusCodes.Status403Forbidden)
                     .ProducesProblem(StatusCodes.Status401Unauthorized)
                     .WithName(ContainersName + "_" + nameof(Containers.StreamLogs));

                
                containers.MapGet("{id}/stats", Containers.GetStats)
                    .WithSummary("Get container stats")
                    .Produces<ContainerStatsView>()
                    .ProducesValidationProblem()
                    .ProducesProblem(StatusCodes.Status404NotFound)
                    .ProducesProblem(StatusCodes.Status403Forbidden)
                    .ProducesProblem(StatusCodes.Status401Unauthorized)
                    .WithName(ContainersName + "_" + nameof(Containers.GetStats));
            }
            var platforms = group.MapGroup("/platforms").WithTags(PlatformsName).RequireAuthorization();
            {
                platforms.MapGet("/", Platforms.List)
                    .WithSummary("List all platforms")
                    .Produces<PlatformsView>()
                    .ProducesValidationProblem()
                    .ProducesProblem(StatusCodes.Status404NotFound)
                    .ProducesProblem(StatusCodes.Status403Forbidden)
                    .ProducesProblem(StatusCodes.Status401Unauthorized)
                    .WithName(PlatformsName + "_" + nameof(Platforms.List));

                platforms.MapGet("/{id}", Platforms.GetById)
                    .WithSummary("Get platform by Id")
                    .Produces<PlatformView>()
                    .ProducesValidationProblem()
                    .ProducesProblem(StatusCodes.Status404NotFound)
                    .ProducesProblem(StatusCodes.Status403Forbidden)
                    .ProducesProblem(StatusCodes.Status401Unauthorized)
                    .WithName(PlatformsName + "_" + nameof(Platforms.GetById));

                platforms.MapGet("/{id}/info", Platforms.GetInfo)
                    .WithSummary("Get platform by Id")
                    .Produces<PlatformView>()
                    .ProducesValidationProblem()
                    .ProducesProblem(StatusCodes.Status404NotFound)
                    .ProducesProblem(StatusCodes.Status403Forbidden)
                    .ProducesProblem(StatusCodes.Status401Unauthorized)
                    .WithName(PlatformsName + "_" + nameof(Platforms.GetInfo));

                platforms.MapPut("/", Platforms.Put)
                    .WithSummary("Create or update a platform")
                    .Produces<PlatformView>()
                    .ProducesValidationProblem()
                    .ProducesProblem(StatusCodes.Status404NotFound)
                    .ProducesProblem(StatusCodes.Status403Forbidden)
                    .ProducesProblem(StatusCodes.Status401Unauthorized)
                    .WithName(PlatformsName + "_" + nameof(Platforms.Put));

                platforms.MapDelete("/", Platforms.Delete)
                    .WithSummary("Delete a platform")
                    .Produces(StatusCodes.Status204NoContent)
                    .ProducesValidationProblem()
                    .ProducesProblem(StatusCodes.Status404NotFound)
                    .ProducesProblem(StatusCodes.Status403Forbidden)
                    .ProducesProblem(StatusCodes.Status401Unauthorized)
                    .WithName(PlatformsName + "_" + nameof(Platforms.Delete));

                platforms.MapGet("{id}/containers", Platforms.ListContainers)
                    .WithSummary("Returns the list of containers of the given platform")
                    .Produces<ContainersInfoView>()
                    .ProducesValidationProblem()
                    .ProducesProblem(StatusCodes.Status404NotFound)
                    .ProducesProblem(StatusCodes.Status403Forbidden)
                    .ProducesProblem(StatusCodes.Status401Unauthorized)
                    .WithName(PlatformsName + "_" + nameof(Platforms.ListContainers));
            }
        }

    }
}