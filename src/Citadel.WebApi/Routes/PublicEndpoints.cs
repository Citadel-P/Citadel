using Domain;
using Hosting.Common;
using Hosting.OpenApi;
using WebApi.Routes.Endpoints;
using WebApi.Routes.Endpoints.Resources;
using WebApi.Routes.Endpoints.Resources.Alerters;
using WebApi.Routes.Endpoints.Resources.Deployments;
using WebApi.Routes.Endpoints.Resources.GitAccounts;
using WebApi.Routes.Endpoints.Resources.GitRepositories;
using WebApi.Routes.Endpoints.Resources.Platforms;
using WebApi.Routes.Endpoints.Resources.Registries;
using WebApi.Routes.Endpoints.Resources.Stacks;
using WebApi.Routes.Endpoints.Resources.Identity.Users;
using WebApi.Routes.Endpoints.Resources.Identity.Roles;
using WebApi.Routes.Endpoints.Resources.Identity.Teams;
using Hosting.Common.Attributes;

namespace WebApi.Routes;

public static class PublicEndpoints
{
    const string ImagesName = nameof(Images);
    const string VolumesName = nameof(Volumes);
    const string NetworksName = nameof(Networks);
    const string PlatformsName = nameof(Platforms);
    const string ContainersName = nameof(Containers);
    const string ActivitiesName = nameof(Activities);
    const string AlertEventsName = nameof(AlertEvents);
    const string AlertRulesName = nameof(AlertRules);
    const string ActorsName = nameof(Actors);
    const string UsersName = nameof(Users);
    const string TeamsName = nameof(Teams);
    const string RolesName = nameof(Roles);
    const string GitAccountsName = nameof(GitAccounts);
    const string GitRepositoriesName = nameof(GitRepositories);
    const string RegistriesName = nameof(Registries);
    const string DeploymentsName = nameof(Deployments);
    const string StacksName = nameof(Stacks);
    const string AuthenticationName = nameof(Authentication);

    public static void MapPublicEndpoints(this WebApplication app)
    {
        var group = app.MapGroup("/api/v1").WithGroupName("v1");
        {
            var auth = group.MapGroup("/authentication").WithTags(AuthenticationName);
            {
                MapAuthEndpoints(auth);
            }
            var actors = group.MapGroup("/actors").WithTags(ActorsName).RequireAuthorization();
            {
                MapActorEndpoints(actors);
            }
            var users = group.MapGroup("/users").WithTags(UsersName).RequireAuthorization();
            {
                MapUserEndpoints(users);
            }
            var teams = group.MapGroup("/teams").WithTags(TeamsName).RequireAuthorization();
            {
                MapTeamEndpoints(teams);
            }
            var roles = group.MapGroup("/roles").WithTags(RolesName).RequireAuthorization();
            {
                MapRoleEndpoints(roles);
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
            var gitAccounts = group.MapGroup("/gitAccounts").WithTags(GitAccountsName).RequireAuthorization();
            {
                MapGitAccountEndpoints(gitAccounts);
            }
            var gitRepositories = group.MapGroup("/gitRepositories").WithTags(GitRepositoriesName).RequireAuthorization();
            {
                MapGitRepositoryEndpoints(gitRepositories);
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
            var deployments = group.MapGroup("/deployments").WithTags(DeploymentsName).RequireAuthorization();
            {
                MapDeploymentEndpoints(deployments);
            }
            var stacks = group.MapGroup("/stacks").WithTags(StacksName).RequireAuthorization();
            {
                MapStackEndpoints(stacks);
            }
            var activities = group.MapGroup("/activities").WithTags(ActivitiesName).RequireAuthorization();
            {
                MapActivityEndpoints(activities);
            }
            var alertEvents = group.MapGroup("/alertEvents").WithTags(AlertEventsName).RequireAuthorization();
            {
                MapAlertEventsEndpoints(alertEvents);
            }
            var alertRules = group.MapGroup("/alertRules").WithTags(AlertRulesName).RequireAuthorization();
            {
                MapAlertRulesEndpoints(alertRules);
            }
        }

        group.ProducesProblem(StatusCodes.Status500InternalServerError);
    }

    private static void MapActorEndpoints(RouteGroupBuilder actors)
    {
        actors.MapGet("{id}", Actors.Get)
            .WithSummary("Get actor by ID")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("getActor");

        actors.MapPatch("{id}/enabled", WebApi.Routes.Endpoints.Actors.PatchEnabled)
            .WithSummary("Enable or disable an actor")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("patchActorEnabled");
    }

    private static void MapUserEndpoints(RouteGroupBuilder users)
    {
        users.MapGet("/", Users.List)
            .WithSummary("Get all users")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("listUsers");

        users.MapGet("/search", Users.Search)
            .WithSummary("Search users for assignment")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("searchUsers");

        users.MapGet("{id}", Users.Get)
            .WithSummary("Get user by ID")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("getUser");

        users.MapPost("/", Users.Create)
            .WithSummary("Create a user")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status409Conflict)
            .WithName("createUser");

        users.MapPatch("{id}", Users.Patch)
            .WithSummary("Update a user")
            .Accepts<PatchUserInput>("application/merge-patch+json", "application/json")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status409Conflict)
            .WithName("updateUser");

        users.MapPost("{id}/roles", Users.AddRole)
            .WithSummary("Assign a role to a user")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status409Conflict)
            .WithName("addUserRole");

        users.MapDelete("{id}/roles/{roleId}", Users.RemoveRole)
            .WithSummary("Remove a role from a user")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("removeUserRole");

        users.MapPost("{id}/resource-accesses", Users.AddResourceAccess)
            .WithSummary("Add resource access override for a user")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status409Conflict)
            .WithName("addUserResourceAccess");

        users.MapDelete("{id}/resource-accesses", Users.RemoveResourceAccess)
            .WithSummary("Remove resource access override for a user")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("removeUserResourceAccess");

        users.MapPost("/rename", Users.Rename)
            .WithSummary("Rename a user")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status409Conflict)
            .WithName("renameUser");

        users.MapDelete("/", Users.Delete)
            .WithSummary("Delete users")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("deleteUsers");
    }

    private static void MapGitAccountEndpoints(RouteGroupBuilder gitAccounts)
    {
        gitAccounts.MapGet("/", GitAccounts.List)
            .WithSummary("Get all git accounts")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("listGitAccounts");

        gitAccounts.MapGet("{id}", GitAccounts.Get)
            .WithSummary("Get git account by ID")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("getGitAccount");

        gitAccounts.MapGet("{id}/_cfg", GitAccounts.GetConfig)
            .WithSummary("Get git account configuration")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("getGitAccountConfig");

        gitAccounts.MapPost("/", GitAccounts.Create)
            .WithSummary("Create a git account")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status409Conflict)
            .WithName("createGitAccount");

        gitAccounts.MapPatch("{id}", GitAccounts.Patch)
            .WithSummary("Update a git account")
            .Accepts<GitAccountInput>("application/merge-patch+json", "application/json")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status409Conflict)
            .WithName("updateGitAccount");

        gitAccounts.MapDelete("/", GitAccounts.Delete)
            .WithSummary("Delete git accounts")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("deleteGitAccounts");
    }

    private static void MapRoleEndpoints(RouteGroupBuilder roles)
    {
        roles.MapGet("/", Roles.List)
            .WithSummary("Get all roles")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("listRoles");

        roles.MapGet("{id}", Roles.Get)
            .WithSummary("Get role by ID")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("getRole");

        roles.MapPost("/", Roles.Create)
            .WithSummary("Create a role")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status409Conflict)
            .WithName("createRole");

        roles.MapPatch("{id}/permissions", Roles.PatchPermissions)
            .WithSummary("Update role permissions")
            .Accepts<PatchRolePermissionsInput>("application/merge-patch+json", "application/json")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("updateRolePermissions");

        roles.MapPost("/rename", Roles.Rename)
            .WithSummary("Rename a role")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status409Conflict)
            .WithName("renameRole");

        roles.MapDelete("/", Roles.Delete)
            .WithSummary("Delete roles")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("deleteRoles");

        roles.MapGet("/permissions/matrix", () =>
            TypedResults.Ok(PermissionMatrixView.Map()))
            .WithSummary("Get the permission matrix (all valid resource capabilities and minimum levels)")
            .WithName("getPermissionMatrix")
            .AllowAnonymous();
    }

    private static void MapGitRepositoryEndpoints(RouteGroupBuilder gitRepositories)
    {
        gitRepositories.MapGet("/", GitRepositories.List)
            .WithSummary("Get all git repositories")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("listGitRepositories");

        gitRepositories.MapGet("{id}", GitRepositories.Get)
            .WithSummary("Get git repository by ID")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("getGitRepository");

        gitRepositories.MapGet("{id}/_cfg", GitRepositories.GetConfig)
            .WithSummary("Get Git repo configuration")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("getGitRepositoryConfig");

        gitRepositories.MapPost("/", GitRepositories.Create)
            .WithSummary("Create a git repository")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status409Conflict)
            .WithName("createGitRepository");

        gitRepositories.MapPatch("{id}", GitRepositories.Patch)
            .WithSummary("Update a git repository")
            .Accepts<PatchGitRepositoryInput>("application/merge-patch+json", "application/json")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status409Conflict)
            .WithName("updateGitRepository");

        gitRepositories.MapPatch("{id}/_metadata", GitRepositories.PatchMetadata)
            .WithSummary("Update git repository metadata")
            .Accepts<PatchResourceMetadata>("application/merge-patch+json", "application/json")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("updateGitRepositoryMetadata");

        gitRepositories.MapPost("/rename", GitRepositories.Rename)
            .WithSummary("Rename a git repository")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status409Conflict)
            .WithName("renameGitRepository");

        gitRepositories.MapDelete("/", GitRepositories.Delete)
            .WithSummary("Delete git repositories")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("deleteGitRepositories");
    }

    private static void MapAuthEndpoints(RouteGroupBuilder auth)
    {
        auth.MapGet("refresh", Authentication.RefreshToken)
            .WithSummary("Request a new access token")
            .WithCookie(Constants.RefreshToken, "Refresh Token", true)
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status404NotFound)
            .WithName("refreshToken");

        auth.MapPost("login", Authentication.Login)
            .WithSummary("Check user credentials and issue an access token on successful login")
            .ProduceCookie(Constants.RefreshToken)
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status429TooManyRequests)
            .RequireRateLimiting("strict-auth")
            .WithName("login");

        auth.MapPost("logout", Authentication.Logout)
            .WithSummary("Log out")
            .WithCookie(Constants.RefreshToken, "Refresh Token", true)
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status404NotFound)
            .WithName("logout");
    }

    private static void MapContainerEndpoints(RouteGroupBuilder containers)
    {
        containers.MapGet("{id}", Containers.Get)
            .WithSummary("Get container by Id")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("getContainer");

        containers.MapGet("{id}/info", Containers.GetInfo)
            .WithSummary("Get basic container details")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("getContainerInfo");

        containers.MapGet("{id}/stats", Containers.GetStats)
            .WithSummary("Get container stats")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("getContainerStats");

        containers.MapGet("{id}/inspect", Containers.Inspect)
            .WithSummary("Inspect a container")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("inspectContainer");

        containers.MapPatch("start", Containers.StartContainers)
            .WithSummary("Starts the given container(s)")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("startContainers");

        containers.MapPatch("stop", Containers.StopContainers)
            .WithSummary("Stops the given container(s)")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("stopContainers");

        containers.MapPatch("pause", Containers.PauseContainers)
            .WithSummary("Pause the given container(s)")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("pauseContainers");

        containers.MapPatch("restart", Containers.RestartContainers)
            .WithSummary("Restarts the given container(s)")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("restartContainers");

        containers.MapPatch("unpause", Containers.UnpauseContainers)
            .WithSummary("Resume a container(s) which has been paused")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("unpauseContainers");

        containers.MapDelete("/", Containers.DeleteContainers)
            .WithSummary("Delete the given container(s)")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("deleteContainers");
    }

    private static void MapPlatformEndpoints(RouteGroupBuilder platforms)
    {
        platforms.MapGet("/", Platforms.List)
            .WithSummary("List all platforms")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("listPlatforms");

        platforms.MapGet("{id}", Platforms.Get)
            .WithSummary("Get platform by Id")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("getPlatfom");

        platforms.MapGet("{id}/containers", Platforms.ListContainers)
            .WithSummary("Returns the list of containers of the given platform")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("listContainers");

        platforms.MapPatch("{id}", Platforms.Patch)
            .WithSummary("Patch a platform")
            .Accepts<PlatformInput>("application/merge-patch+json", "application/json")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status409Conflict)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("updatePlatform");

        platforms.MapPost("/", Platforms.Create)
            .WithSummary("Create a platform")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("createPlatform");

        platforms.MapDelete("/", Platforms.Delete)
            .WithSummary("Delete platforms")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("deletePlatforms");

    }

    private static void MapRegistryEndpoints(RouteGroupBuilder registries)
    {
        registries.MapGet("/", Registries.List)
            .WithSummary("Get all registries")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("listRegistries");

        registries.MapGet("{id}", Registries.Get)
            .WithSummary("Get registry by ID")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("getRegistry");

        registries.MapGet("{id}/_cfg", Registries.GetConfig)
            .WithSummary("Get registry configuration")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("getRegistryConfig");

        registries.MapPost("/", Registries.Create)
            .WithSummary("Create a registry")
            .WithDescription($"A discriminator should be provided in the request, this discriminator is based on {nameof(RegistryType)} enum ")
            .WithExample(RegistryType.Azure.ToString(), Examples.Registries.Create.CreateAzureRegistryExample())
            .WithExample(RegistryType.AWS.ToString(), Examples.Registries.Create.CreateAwsRegistryExample())
            .WithExample(RegistryType.Gitlab.ToString(), Examples.Registries.Create.CreateGitlabRegistryExample())
            .WithExample(RegistryType.DockerHub.ToString(), Examples.Registries.Create.CreateDockerHubRegistryExample())
            .WithExample(RegistryType.GitHub.ToString(), Examples.Registries.Create.CreateGitHubRegistryExample())
            .WithExample(RegistryType.Custom.ToString(), Examples.Registries.Create.CreateCustomRegistryExample())
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("createRegistry");

        registries.MapPatch("{id}", Registries.Patch)
            .WithSummary("Update a registry")
            .WithDescription($"A discriminator should be provided in the request, this discriminator is based on {nameof(RegistryType)} enum")
            .Accepts<PatchRegistryInput>("application/merge-patch+json", "application/json")
            .WithExample(RegistryType.Azure.ToString(), Examples.Registries.Update.UpdateAzureRegistryExample())
            .WithExample(RegistryType.AWS.ToString(), Examples.Registries.Update.UpdateAwsRegistryExample())
            .WithExample(RegistryType.Gitlab.ToString(), Examples.Registries.Update.UpdateGitlabRegistryExample())
            .WithExample(RegistryType.DockerHub.ToString(), Examples.Registries.Update.UpdateDockerHubRegistryExample())
            .WithExample(RegistryType.GitHub.ToString(), Examples.Registries.Update.UpdateGitHubRegistryExample())
            .WithExample(RegistryType.Custom.ToString(), Examples.Registries.Update.UpdateCustomRegistryExample())
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status409Conflict)
            .WithName("updateRegistry");

        registries.MapPatch("{id}/_metadata", Registries.PatchMetadata)
            .WithSummary("Update registry metadata")
            .Accepts<PatchResourceMetadata>("application/merge-patch+json", "application/json")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status409Conflict)
            .WithName("updateRegistryMetadata");

        registries.MapPost("/rename", Registries.Rename)
            .WithSummary("Rename registry")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status409Conflict)
            .WithName("renameRegistry");

        registries.MapDelete("/", Registries.Delete)
            .WithSummary("Delete registries")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("deleteRegistries");
    }

    private static void MapImageEndpoints(RouteGroupBuilder images)
    {
        images.MapGet("{platformId}", Images.ListImages)
            .WithSummary("Get all local images for the given platform")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("listImages");

        images.MapGet("/{registryName}/repositories", Images.GetExternalRepositories)
            .WithSummary("List external repositories of the given registry")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("getExternalRepositories");

        images.MapGet("/ghcr/{registryName}/{packageName}/versions", Images.GetGhcrPackageVersions)
            .WithSummary("List versions of GHCR package")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("getGhcrPackageVersions");

        images.MapGet("/dockerhub/{registryName}/repositories", Images.GetDockerHubRepositories)
            .WithSummary("List DockerHub repositories")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("getDockerHubRepositories");

        images.MapGet("/dockerhub/{registryName}/{repositoryName}/tags", Images.GetDockerHubRepositoryTags)
            .WithSummary("List DockerHub repository tags")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("getDockerHubRepositoryTags");

        images.MapGet("{platformId}/{imageId}", Images.Inspect)
           .WithSummary("Inspect an image")
           .ProducesValidationProblem()
           .ProducesProblem(StatusCodes.Status403Forbidden)
           .ProducesProblem(StatusCodes.Status401Unauthorized)
           .ProducesProblem(StatusCodes.Status404NotFound)
           .WithName("inspectImage");

        images.MapGet("{platformId}/{imageId}/_ports", Images.GetExposedPorts)
           .WithSummary("Get image ports")
           .ProducesValidationProblem()
           .ProducesProblem(StatusCodes.Status403Forbidden)
           .ProducesProblem(StatusCodes.Status401Unauthorized)
           .ProducesProblem(StatusCodes.Status404NotFound)
           .WithName("getExposedPorts");

        images.MapPost("/pull", Images.PullImage)
            .WithSummary("Pull an image from a registry and streams execution logs in real time.")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("pullImage");

        images.MapDelete("/", Images.Delete)
            .WithSummary("Remove an image(s), along with any untagged parent images that were referenced by that image")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("deleteImages");
    }

    private static void MapNetworkEndpoints(RouteGroupBuilder networks)
    {
        networks.MapGet("{platformId}", Networks.List)
           .WithSummary("List all networks")
           .ProducesValidationProblem()
           .ProducesProblem(StatusCodes.Status403Forbidden)
           .ProducesProblem(StatusCodes.Status401Unauthorized)
           .ProducesProblem(StatusCodes.Status409Conflict)
           .WithName("listNetworks");

        networks.MapGet("{platformId}/{networkId}", Networks.Inspect)
           .WithSummary("Inspect a network")
           .ProducesValidationProblem()
           .ProducesProblem(StatusCodes.Status403Forbidden)
           .ProducesProblem(StatusCodes.Status401Unauthorized)
           .ProducesProblem(StatusCodes.Status409Conflict)
           .WithName("inspectNetwork");

        networks.MapPost("/", Networks.Create)
            .WithSummary("Create a network")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status409Conflict)
            .WithName("createNetwork");

        networks.MapDelete("/", Networks.Delete)
            .WithSummary("Delete a network(s)")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status404NotFound)
            .WithName("deleteNetworks");
    }

    private static void MapVolumeEndpoints(RouteGroupBuilder volumes)
    {
        volumes.MapGet("{platformId}", Volumes.List)
           .WithSummary("List all volumes")
           .ProducesValidationProblem()
           .ProducesProblem(StatusCodes.Status403Forbidden)
           .ProducesProblem(StatusCodes.Status401Unauthorized)
           .ProducesProblem(StatusCodes.Status409Conflict)
           .WithName("listVolumes");

        volumes.MapGet("{platformId}/{name}", Volumes.Inspect)
           .WithSummary("Inspect a volume")
           .ProducesValidationProblem()
           .ProducesProblem(StatusCodes.Status403Forbidden)
           .ProducesProblem(StatusCodes.Status401Unauthorized)
           .ProducesProblem(StatusCodes.Status409Conflict)
           .WithName("inspectVolume");

        volumes.MapPost("/", Volumes.Create)
            .WithSummary("Create a volume")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status409Conflict)
            .WithName("createVolume");

        volumes.MapDelete("/", Volumes.Delete)
            .WithSummary("Delete a volume(s)")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status404NotFound)
            .WithName("deleteVolumes");
    }

    private static void MapDeploymentEndpoints(RouteGroupBuilder deployment)
    {
        deployment.MapGet("/", Deployments.List)
            .WithSummary("List all deployments")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status404NotFound)
            .WithName("listDeployments");

        deployment.MapGet("/{deploymentId}", Deployments.Get)
            .WithSummary("Get deployment by Id")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status404NotFound)
            .WithName("getDeployment");

        deployment.MapGet("/{deploymentId}/_cfg", Deployments.GetConfig)
            .WithSummary("Get deployment configuration")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status404NotFound)
            .WithName("getDeploymentConfig");

        deployment.MapPost("/", Deployments.Create)
            .WithSummary("Create a deployment")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("createDeployment");

        deployment.MapPost("/rename", Deployments.Rename)
            .WithSummary("Rename deployment")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("renameDeployment");

        deployment.MapPatch("{id}", Deployments.Patch)
            .WithSummary("Update a deployment")
            .Accepts<PatchDeploymentInput>("application/merge-patch+json", "application/json")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status409Conflict)
            .WithName("updateDeployment");

        deployment.MapPatch("{id}/_metadata", Deployments.PatchMetadata)
            .WithSummary("Update deployment metadata")
            .Accepts<PatchResourceMetadata>("application/merge-patch+json", "application/json")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status409Conflict)
            .WithName("updateDeploymentMetadata");

        deployment.MapDelete("/", Deployments.Delete)
            .WithSummary("Delete deployments")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("deleteDeployments");

        deployment.MapPost("/apply", Deployments.ApplyDeployment)
            .WithSummary("Apply a deployment and streams execution logs in real time.")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("applyDeployment");

        deployment.MapPost("/resume", Deployments.Resume)
            .WithSummary("Resume deployments")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("resumeDeployments");

        deployment.MapPost("/pause", Deployments.Pause)
            .WithSummary("Pause deployments")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("pauseDeployments");

        deployment.MapPost("/restart", Deployments.Restart)
            .WithSummary("Restart deployments")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("restartDeployments");

        deployment.MapPost("/stop", Deployments.Stop)
            .WithSummary("Stop deployments")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("stopDeployments");

        deployment.MapPost("/start", Deployments.Start)
            .WithSummary("Start deployments")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("startDeployments");
    }

    private static void MapStackEndpoints(RouteGroupBuilder stacks)
    {
        stacks.MapGet("/", Stacks.List)
            .WithSummary("List all stacks")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status404NotFound)
            .WithName("listStacks");

        stacks.MapGet("/{stackId}", Stacks.Get)
            .WithSummary("Get stack by Id")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status404NotFound)
            .WithName("getStack");

        stacks.MapGet("/{stackId}/_cfg", Stacks.GetConfig)
            .WithSummary("Get stack configuration")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status404NotFound)
            .WithName("getStackConfig");

        stacks.MapGet("/{stackId}/releases", Stacks.ListReleases)
            .WithSummary("List stack releases")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status404NotFound)
            .WithName("listStackReleases");

        stacks.MapPost("/", Stacks.Create)
            .WithSummary("Create a stack")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status409Conflict)
            .WithName("createStack");

        stacks.MapPost("/rename", Stacks.Rename)
            .WithSummary("Rename a stack")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status409Conflict)
            .WithName("renameStack");

        stacks.MapPatch("{id}", Stacks.Patch)
            .WithSummary("Update a stack")
            .Accepts<PatchStackInput>("application/merge-patch+json", "application/json")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status409Conflict)
            .WithName("updateStack");

        stacks.MapPatch("{id}/_metadata", Stacks.PatchMetadata)
            .WithSummary("Update stack metadata")
            .Accepts<PatchResourceMetadata>("application/merge-patch+json", "application/json")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status409Conflict)
            .WithName("updateStackMetadata");

        stacks.MapDelete("/", Stacks.Delete)
            .WithSummary("Delete stacks")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("deleteStacks");
    }

    private static void MapActivityEndpoints(RouteGroupBuilder activities)
    {
        activities.MapGet("{id}", Activities.Get)
            .WithSummary("Get activity by id")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("getActivity");

        activities.MapGet("/", Activities.List)
            .WithSummary("List activity events")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("listActivities");
    }

    private static void MapAlertRulesEndpoints(RouteGroupBuilder alertRules)
    {
        var rules = alertRules.MapGroup("/");

        rules.MapGet("{id}", AlertRules.GetRule)
            .WithSummary("Get alert rule by id")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("getAlertRule");

        rules.MapGet("{id}/_cfg", AlertRules.GetConfig)
            .WithSummary("Get alert rule configuration")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("getAlertRuleConfig");

        rules.MapGet("/", AlertRules.ListRules)
            .WithSummary("List alert rules")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("listAlertRules");

        rules.MapPost("/", AlertRules.CreateRule)
            .WithSummary("Create an alert rule")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("createAlertRule");

        rules.MapPost("/rename", AlertRules.RenameRule)
            .WithSummary("Rename an alert rule")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("renameAlertRule");

        rules.MapPatch("{id}", AlertRules.PatchRule)
            .WithSummary("Update an alert rule")
            .Accepts<PatchAlertRuleInput>("application/merge-patch+json", "application/json")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("updateAlertRule");

        rules.MapPatch("{id}/_metadata", AlertRules.PatchRuleMetadata)
            .WithSummary("Update alert rule metadata")
            .Accepts<PatchResourceMetadata>("application/merge-patch+json", "application/json")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("updateAlertRuleMetadata");

        rules.MapDelete("/", AlertRules.DeleteRules)
            .WithSummary("Delete alert rules")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("deleteAlertRules");

        var channels = alertRules.MapGroup("/channels");

        channels.MapGet("{id}", AlertRules.GetChannel)
            .WithSummary("Get alert channel by id")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("getAlertChannel");

        channels.MapGet("/", AlertRules.ListChannels)
            .WithSummary("List alert channels")
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("listAlertChannels");

        channels.MapPost("/", AlertRules.CreateChannel)
            .WithSummary("Create an alert channel")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("createAlertChannel");

        channels.MapPost("/verify", AlertRules.VerifyChannel)
            .WithSummary("Verify an alert channel URL")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status400BadRequest)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("verifyAlertChannel");

        channels.MapPatch("{id}", AlertRules.PatchChannel)
            .WithSummary("Update an alert channel")
            .Accepts<AlertChannelInput>("application/merge-patch+json", "application/json")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("updateAlertChannel");

        channels.MapDelete("/", AlertRules.DeleteChannels)
            .WithSummary("Delete alert channels")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("deleteAlertChannels");
    }

    private static void MapAlertEventsEndpoints(RouteGroupBuilder alertEvents)
    {
        alertEvents.MapGet("{id}", AlertEvents.Get)
            .WithSummary("Get alert event by id")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("getAlertEvent");

        alertEvents.MapGet("/", AlertEvents.List)
            .WithSummary("List alert events")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("listAlertEvents");

        alertEvents.MapGet("/unresolved-count", AlertEvents.GetUnresolvedCount)
            .WithSummary("Get unresolved alert event count")
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("getUnresolvedAlertEventsCount");

        alertEvents.MapPost("/acknowledge", AlertEvents.Acknowledge)
            .WithSummary("Acknowledge alert events in bulk")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status400BadRequest)
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("acknowledgeAlertEvents");

        alertEvents.MapPost("/resolve", AlertEvents.Resolve)
            .WithSummary("Resolve alert events in bulk")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status400BadRequest)
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("resolveAlertEvents");
    }

    private static void MapTeamEndpoints(RouteGroupBuilder teams)
    {
        teams.MapGet("/", Teams.List)
            .WithSummary("Get all teams")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("listTeams");

        teams.MapGet("/search", Teams.Search)
            .WithSummary("Search teams for assignment")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("searchTeams");

        teams.MapGet("{id}", Teams.Get)
            .WithSummary("Get team by ID")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("getTeam");

        teams.MapPost("/", Teams.Create)
            .WithSummary("Create a team")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status409Conflict)
            .WithName("createTeam");

        teams.MapPatch("{id}", Teams.Patch)
            .WithSummary("Update a team")
            .Accepts<PatchTeamInput>("application/merge-patch+json", "application/json")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("updateTeam");

        teams.MapPost("{id}/roles", Teams.AddRole)
            .WithSummary("Assign a role to a team")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status409Conflict)
            .WithName("addTeamRole");

        teams.MapDelete("{id}/roles/{roleId}", Teams.RemoveRole)
            .WithSummary("Remove a role from a team")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("removeTeamRole");

        teams.MapPost("{id}/members", Teams.AddMember)
            .WithSummary("Add a member to a team")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status409Conflict)
            .WithName("addTeamMember");

        teams.MapDelete("{id}/members/{userId}", Teams.RemoveMember)
            .WithSummary("Remove a member from a team")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("removeTeamMember");

        teams.MapPost("{id}/resource-accesses", Teams.AddResourceAccess)
            .WithSummary("Add resource access override for a team")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status409Conflict)
            .WithName("addTeamResourceAccess");

        teams.MapDelete("{id}/resource-accesses", Teams.RemoveResourceAccess)
            .WithSummary("Remove resource access override for a team")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("removeTeamResourceAccess");

        teams.MapPost("/rename", Teams.Rename)
            .WithSummary("Rename a team")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status409Conflict)
            .WithName("renameTeam");

        teams.MapDelete("/", Teams.Delete)
            .WithSummary("Delete teams")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("deleteTeams");
    }
}
