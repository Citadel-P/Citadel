using WebApi.Controllers.V1.Resources.Containers;
using WebApi.Routes.Endpoints;

namespace WebApi.Routes;

public static class InternalEndpoints
{
    const string ContainersName = nameof(Containers);
    const string PlatformsName = nameof(Platforms);

    public static void MapInternalEndpoints(this WebApplication app)
    {
        var group = app.MapGroup("/api/v1/internal").WithGroupName(Constants.InternalApiV1);
        {
            var containers = group.MapGroup("/containers").WithTags(ContainersName);
            {
                containers.MapPut("_info", Containers.ContainerInfo)
                    .WithSummary("Update or create the containers info entry")
                    .Produces(StatusCodes.Status204NoContent)
                    .ProducesValidationProblem()
                    .ProducesProblem(StatusCodes.Status404NotFound)
                    .ProducesProblem(StatusCodes.Status403Forbidden)
                    .ProducesProblem(StatusCodes.Status401Unauthorized)
                    .WithName(ContainersName + "_" + nameof(Containers.ContainerInfo));

                containers.MapPut("_event", Containers.ContainerEvent)
                    .WithSummary("Handles the event issued from a container")
                    .Produces(StatusCodes.Status204NoContent)
                    .ProducesValidationProblem()
                    .ProducesProblem(StatusCodes.Status404NotFound)
                    .ProducesProblem(StatusCodes.Status403Forbidden)
                    .ProducesProblem(StatusCodes.Status401Unauthorized)
                    .WithName(ContainersName + "_" + nameof(Containers.ContainerEvent));

                containers.MapPost("_logs", Containers.ContainerLogs)
                    .WithSummary("Request to start (or stop) streaming container logs")
                    .Produces(StatusCodes.Status204NoContent)
                    .ProducesValidationProblem()
                    .ProducesProblem(StatusCodes.Status404NotFound)
                    .ProducesProblem(StatusCodes.Status403Forbidden)
                    .ProducesProblem(StatusCodes.Status401Unauthorized)
                    .WithName(ContainersName + "_" + nameof(Containers.ContainerLogs));
            }

            var platforms = group.MapGroup("/platforms").WithTags(PlatformsName);
            {
                platforms.MapPut("_info", Platforms.SystemInfo)
                    .WithSummary("Updates a new system info entry")
                    .Produces(StatusCodes.Status204NoContent)
                    .ProducesValidationProblem()
                    .ProducesProblem(StatusCodes.Status404NotFound)
                    .ProducesProblem(StatusCodes.Status403Forbidden)
                    .ProducesProblem(StatusCodes.Status401Unauthorized)
                    .WithName(PlatformsName + "_" + nameof(Platforms.SystemInfo));
            }
        }
    }
}

