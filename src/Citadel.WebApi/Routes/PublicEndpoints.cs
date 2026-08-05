using Application.Services.Backups;
using Application.Features.Search.Models;
using Application.Features.Identity.Auth.Models;
using Domain;
using Domain.Entities.Deployments;
using Hosting.Common;
using Hosting.OpenApi;
using Mediator;
using Microsoft.AspNetCore.Mvc;
using WebApi.Routes.Endpoints;
using WebApi.Routes.Endpoints.Resources;
using WebApi.Routes.Endpoints.Resources.Automation;
using WebApi.Routes.Endpoints.Resources.Backups;
using WebApi.Routes.Endpoints.Resources.Builds;
using WebApi.Routes.Endpoints.Resources.Alerters;
using WebApi.Routes.Endpoints.Resources.Deployments;
using WebApi.Routes.Endpoints.Resources.GitAccounts;
using WebApi.Routes.Endpoints.Resources.GitRepositories;
using WebApi.Routes.Endpoints.Resources.Identity.Roles;
using WebApi.Routes.Endpoints.Resources.Identity.Teams;
using WebApi.Routes.Endpoints.Resources.Identity.Users;
using WebApi.Routes.Endpoints.Resources.Identity.Profile;
using WebApi.Routes.Endpoints.Resources.Identity.Mfa;
using WebApi.Routes.Endpoints.Resources.Identity.Setup;
using WebApi.Routes.Endpoints.Resources.Licensing;
using WebApi.Routes.Endpoints.Resources.Oidc;
using WebApi.Routes.Endpoints.Resources.Platforms;
using WebApi.Routes.Endpoints.Resources.Registries;
using WebApi.Routes.Endpoints.Resources.ResourceBindings;
using WebApi.Routes.Endpoints.Resources.Stacks;
using WebApi.Routes.Endpoints.Resources.Tags;
using WebApi.Routes.Endpoints.Resources.Volumes;

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
    const string ResourceBindingsName = nameof(ResourceBindings);
    const string OidcProvidersName = "OidcProviders";
    const string AutomationActionsName = "AutomationActions";
    const string BackupRepositoriesName = "BackupRepositories";
    const string BackupPoliciesName = "BackupPolicies";
    const string BackupRunsName = "BackupRuns";
    const string BackupRestoreRunsName = "BackupRestoreRuns";
    const string BuildProjectsName = "BuildProjects";
    const string BuildAgentPoolsName = "BuildAgentPools";
    const string BuildRunsName = "BuildRuns";
    const string TagsName = nameof(Tags);
    const string AuthenticationName = nameof(Authentication);
    const string SetupName = nameof(Setup);
    const string ProfileName = "Profile";
    const string ApplicationName = "Application";
    const string LicenseName = "License";
    const string LookupName = nameof(Lookup);
    const string SearchName = "Search";

    public static void MapPublicEndpoints(this WebApplication app)
    {
        var group = app.MapGroup("/api/v1").WithGroupName("v1");
        {
            var setup = group.MapGroup("/setup").WithTags(SetupName);
            {
                MapSetupEndpoints(setup);
            }
            var auth = group.MapGroup("/authentication").WithTags(AuthenticationName);
            {
                MapAuthEndpoints(auth);
            }
            var application = group.MapGroup("/application").WithTags(ApplicationName).RequireAuthorization();
            {
                MapApplicationEndpoints(application);
            }
            var profile = group.MapGroup("/profile").WithTags(ProfileName).RequireAuthorization();
            {
                MapProfileEndpoints(profile);
            }
            var license = group.MapGroup("/license").WithTags(LicenseName).RequireAuthorization();
            {
                MapLicenseEndpoints(license);
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
            var resourceBindings = group.MapGroup("/resourceBindings").WithTags(ResourceBindingsName).RequireAuthorization();
            {
                MapResourceBindingEndpoints(resourceBindings);
                MapResourceBindingSecretEndpoints(resourceBindings);
            }
            var oidcProviders = group.MapGroup("/oidcProviders").WithTags(OidcProvidersName).RequireAuthorization();
            {
                MapOidcProviderEndpoints(oidcProviders);
            }
            var automationActions = group.MapGroup("/automation/actions").WithTags(AutomationActionsName).RequireAuthorization();
            {
                MapAutomationActionEndpoints(automationActions);
            }
            var backupRepositories = group.MapGroup("/backupRepositories").WithTags(BackupRepositoriesName).RequireAuthorization();
            {
                MapBackupRepositoryEndpoints(backupRepositories);
            }
            var backupPolicies = group.MapGroup("/backupPolicies").WithTags(BackupPoliciesName).RequireAuthorization();
            {
                MapBackupPolicyEndpoints(backupPolicies);
            }
            var backupRuns = group.MapGroup("/backupRuns").WithTags(BackupRunsName).RequireAuthorization();
            {
                MapBackupRunEndpoints(backupRuns);
            }
            var backupRestoreRuns = group.MapGroup("/backupRestoreRuns").WithTags(BackupRestoreRunsName).RequireAuthorization();
            {
                MapBackupRestoreRunEndpoints(backupRestoreRuns);
            }
            var buildProjects = group.MapGroup("/buildProjects").WithTags(BuildProjectsName).RequireAuthorization();
            {
                MapBuildProjectEndpoints(buildProjects);
            }
            var buildAgentPools = group.MapGroup("/buildAgentPools").WithTags(BuildAgentPoolsName).RequireAuthorization();
            {
                MapBuildAgentPoolEndpoints(buildAgentPools);
            }
            var buildRuns = group.MapGroup("/buildRuns").WithTags(BuildRunsName).RequireAuthorization();
            {
                MapBuildRunEndpoints(buildRuns);
            }
            var tags = group.MapGroup("/tags").WithTags(TagsName).RequireAuthorization();
            {
                MapTagEndpoints(tags);
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
            var lookup = group.MapGroup("/lookup").WithTags(LookupName).RequireAuthorization();
            {
                MapLookupEndpoints(lookup);
            }
            var search = group.MapGroup("/search").WithTags(SearchName).RequireAuthorization();
            {
                MapSearchEndpoints(search);
            }
        }

        group.ProducesProblem(StatusCodes.Status500InternalServerError);

        app.MapPost("/listener/{authType}/{resourceType}/{id:guid}/{execution}", WebhookListener.Receive)
            .WithTags("WebhookListener")
            .WithSummary("Receive a provider webhook delivery")
            .AllowAnonymous()
            .RequireRateLimiting("webhook-listener")
            .WithName("receiveWebhook");
    }

    private static void MapSetupEndpoints(RouteGroupBuilder setup)
    {
        setup.MapGet("status", Setup.GetStatus)
            .WithSummary("Get first-run setup status")
            .Produces<SetupStatusView>()
            .ProducesProblem(StatusCodes.Status503ServiceUnavailable)
            .WithName("getSetupStatus");

        setup.MapPost("initialize", Setup.Initialize)
            .WithSummary("Create the initial administrator")
            .Produces<LoginResponse>()
            .ProducesProblem(StatusCodes.Status400BadRequest)
            .ProducesProblem(StatusCodes.Status409Conflict)
            .ProducesProblem(StatusCodes.Status503ServiceUnavailable)
            .RequireRateLimiting("strict-auth")
            .WithName("initializeCitadel");
    }

    private static void MapApplicationEndpoints(RouteGroupBuilder application)
    {
        application.MapGet("info", ApplicationInfo.Get)
            .WithSummary("Get application information")
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("getApplicationInfo");
    }

    private static void MapProfileEndpoints(RouteGroupBuilder profile)
    {
        profile.MapGet("/", WebApi.Routes.Endpoints.Profile.Get)
            .WithSummary("Get current profile")
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("getCurrentProfile");

        profile.MapPatch("/", WebApi.Routes.Endpoints.Profile.Update)
            .WithSummary("Update current profile")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("updateCurrentProfile");

        profile.MapGet("preferences", WebApi.Routes.Endpoints.Profile.GetPreferences)
            .WithSummary("Get current profile preferences")
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("getProfilePreferences");

        profile.MapPatch("preferences", WebApi.Routes.Endpoints.Profile.PatchPreferences)
            .WithSummary("Patch current profile preferences")
            .Accepts<PatchUserPreferencesInput>("application/merge-patch+json", "application/json")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("patchProfilePreferences");

        profile.MapPost("change-password", WebApi.Routes.Endpoints.Profile.ChangePassword)
            .WithSummary("Change current password")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("changeCurrentPassword");

        profile.MapGet("sessions", WebApi.Routes.Endpoints.Profile.ListSessions)
            .WithSummary("List current profile sessions")
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("listProfileSessions");

        profile.MapDelete("sessions/{sessionId:guid}", WebApi.Routes.Endpoints.Profile.RevokeSession)
            .WithSummary("Revoke profile session")
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("revokeProfileSession");

        profile.MapDelete("sessions", WebApi.Routes.Endpoints.Profile.RevokeOtherSessions)
            .WithSummary("Revoke other profile sessions")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("revokeOtherProfileSessions");

        profile.MapGet("mfa", Mfa.GetProfileMfaStatus)
            .WithSummary("Get current profile MFA status")
            .Produces<ProfileMfaStatusView>()
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("getProfileMfaStatus");

        profile.MapPost("mfa/setup", Mfa.StartProfileMfaSetup)
            .WithSummary("Start current profile MFA setup")
            .Produces<ProfileMfaSetupView>()
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status429TooManyRequests)
            .RequireRateLimiting("strict-auth")
            .WithName("startProfileMfaSetup");

        profile.MapPost("mfa/setup/confirm", Mfa.ConfirmProfileMfaSetup)
            .WithSummary("Confirm current profile MFA setup")
            .Produces<ProfileMfaRecoveryCodesView>()
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status429TooManyRequests)
            .RequireRateLimiting("strict-auth")
            .WithName("confirmProfileMfaSetup");

        profile.MapPost("mfa/disable", Mfa.DisableProfileMfa)
            .WithSummary("Disable current profile MFA")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status429TooManyRequests)
            .RequireRateLimiting("strict-auth")
            .WithName("disableProfileMfa");

        profile.MapPost("mfa/recovery-codes", Mfa.RegenerateProfileMfaRecoveryCodes)
            .WithSummary("Regenerate current profile MFA recovery codes")
            .Produces<ProfileMfaRecoveryCodesView>()
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status429TooManyRequests)
            .RequireRateLimiting("strict-auth")
            .WithName("regenerateProfileMfaRecoveryCodes");
    }

    private static void MapLicenseEndpoints(RouteGroupBuilder license)
    {
        license.MapGet("entitlements", WebApi.Routes.Endpoints.License.GetEntitlements)
            .WithSummary("Get effective license entitlements")
            .Produces<LicenseEntitlementsView>()
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("getLicenseEntitlements");

        license.MapGet("/", WebApi.Routes.Endpoints.License.Get)
            .WithSummary("Get license status")
            .Produces<LicenseView>()
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .WithName("getLicense");

        license.MapPost("/", WebApi.Routes.Endpoints.License.Install)
            .WithSummary("Install or replace license")
            .Produces<LicenseView>()
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .WithName("installLicense");

        license.MapDelete("/", WebApi.Routes.Endpoints.License.Remove)
            .WithSummary("Remove installed license")
            .Produces<LicenseView>()
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .WithName("removeLicense");

        license.MapGet("request", WebApi.Routes.Endpoints.License.GetRequest)
            .WithSummary("Get license request details")
            .Produces<LicenseRequestView>()
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .WithName("getLicenseRequest");
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

        users.MapDelete("{id:guid}/mfa", Mfa.ResetUserMfa)
            .WithSummary("Reset user MFA")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("resetUserMfa");
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

        gitRepositories.MapGet("{id}/tags", Tags.GetGitRepositoryTags)
            .WithSummary("Get git repository tags")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("getGitRepositoryTags");

        gitRepositories.MapPut("{id}/tags", Tags.ReplaceGitRepositoryTags)
            .WithSummary("Replace git repository tags")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status400BadRequest)
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("replaceGitRepositoryTags");

        gitRepositories.MapGet("{id}/_cfg", GitRepositories.GetConfig)
            .WithSummary("Get Git repo configuration")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("getGitRepositoryConfig");

        gitRepositories.MapGet("{id}/refs", GitRepositories.GetRefs)
            .WithSummary("Get synced Git repository refs")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("getGitRepositoryRefs");

        gitRepositories.MapGet("{id}/files", GitRepositories.ListFiles)
            .WithSummary("List a directory at a Git repository commit")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status400BadRequest)
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status409Conflict)
            .ProducesProblem(StatusCodes.Status502BadGateway)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("listGitRepositoryDirectory");

        gitRepositories.MapGet("{id}/files/content", GitRepositories.GetFileContent)
            .WithSummary("Read a text file at a Git repository commit")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status400BadRequest)
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status409Conflict)
            .ProducesProblem(StatusCodes.Status502BadGateway)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("getGitRepositoryFileContent");

        gitRepositories.MapGet("{id}/compare", GitRepositories.CompareCommits)
            .WithSummary("Compare two Git repository commits")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status400BadRequest)
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status409Conflict)
            .ProducesProblem(StatusCodes.Status502BadGateway)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("compareGitRepositoryCommits");

        gitRepositories.MapGet("{id}/branches", GitRepositories.DiscoverBranches)
            .WithSummary("Discover remote Git repository branches")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("discoverGitRepositoryBranches");

        gitRepositories.MapGet("{id}/compose-projects", GitRepositories.DiscoverComposeProjects)
            .WithSummary("Discover compose projects in a Git repository branch")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("discoverGitRepositoryComposeProjects");

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

        gitRepositories.MapPost("{id}/sync", GitRepositories.Sync)
            .WithSummary("Sync a git repository")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("syncGitRepository");

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

    private static void MapResourceBindingEndpoints(RouteGroupBuilder resourceBindings)
    {
        resourceBindings.MapGet("global", ResourceBindings.GetGlobal)
            .WithName("getGlobalResourceBindings")
            .WithSummary("Get global bindings");

        resourceBindings.MapPost("global", ResourceBindings.CreateGlobal)
            .WithName("createGlobalResourceBinding")
            .WithSummary("Create a global binding");

        resourceBindings.MapPatch("global", ResourceBindings.UpdateGlobal)
            .WithName("updateGlobalResourceBinding")
            .WithSummary("Update a global binding");

        resourceBindings.MapDelete("global/{id:guid}", ResourceBindings.DeleteGlobal)
            .WithName("deleteGlobalResourceBinding")
            .WithSummary("Delete a global binding");

        resourceBindings.MapGet("{scope}/{resourceId:guid}", ResourceBindings.GetResource)
            .WithName("getResourceBindings")
            .WithSummary("Get resource bindings");

        resourceBindings.MapPost("{scope}/{resourceId:guid}", ResourceBindings.CreateResource)
            .WithName("createResourceBinding")
            .WithSummary("Create a resource binding");

        resourceBindings.MapPatch("{scope}/{resourceId:guid}", ResourceBindings.UpdateResource)
            .WithName("updateResourceBinding")
            .WithSummary("Update a resource binding");

        resourceBindings.MapDelete("{scope}/{resourceId:guid}/{id:guid}", ResourceBindings.DeleteResource)
            .WithName("deleteResourceBinding")
            .WithSummary("Delete a resource binding");
    }

    private static void MapResourceBindingSecretEndpoints(RouteGroupBuilder resourceBindings)
    {
        resourceBindings.MapGet("secrets", ResourceBindings.ListSecrets)
            .WithName("listSecretDefinitions")
            .WithSummary("List secret definitions");

        resourceBindings.MapPost("secrets", ResourceBindings.CreateInternalSecret)
            .WithName("createInternalSecret")
            .WithSummary("Create an internal encrypted secret");

        resourceBindings.MapPost("secrets/external", ResourceBindings.CreateExternalSecret)
            .WithName("createExternalSecret")
            .WithSummary("Create an external secret definition");

        resourceBindings.MapPatch("secrets/external/{id:guid}", ResourceBindings.UpdateExternalSecret)
            .Accepts<UpdateExternalSecretInput>("application/merge-patch+json", "application/json")
            .WithName("updateExternalSecret")
            .WithSummary("Update an external secret definition");

        resourceBindings.MapDelete("secrets/{id:guid}", ResourceBindings.DeleteSecretDefinition)
            .WithName("deleteSecretDefinition")
            .WithSummary("Delete an unused stored secret definition");

        resourceBindings.MapPost("secrets/external/test", ResourceBindings.TestExternalSecret)
            .WithName("testExternalSecret")
            .WithSummary("Test an external secret reference");

        resourceBindings.MapPost("secret-providers/vault-kv2/test", ResourceBindings.TestVaultKvV2SecretProviderConnection)
            .WithName("testVaultKvV2SecretProviderConnection")
            .WithSummary("Test a Vault-compatible KV v2 secret provider connection");

        resourceBindings.MapGet("secret-providers", ResourceBindings.ListSecretProviders)
            .WithName("listSecretProviders")
            .WithSummary("List secret providers");

        resourceBindings.MapPost("secret-providers/vault-kv2", ResourceBindings.CreateVaultKvV2SecretProvider)
            .WithName("createVaultKvV2SecretProvider")
            .WithSummary("Create a Vault-compatible KV v2 secret provider");

        resourceBindings.MapPatch("secret-providers/vault-kv2/{id:guid}", ResourceBindings.UpdateVaultKvV2SecretProvider)
            .Accepts<UpdateVaultKvV2SecretProviderInput>("application/merge-patch+json", "application/json")
            .WithName("updateVaultKvV2SecretProvider")
            .WithSummary("Update a Vault-compatible KV v2 secret provider");

        resourceBindings.MapDelete("secret-providers/{id:guid}", ResourceBindings.DeleteSecretProvider)
            .WithName("deleteSecretProvider")
            .WithSummary("Delete a secret provider");
    }

    private static void MapTagEndpoints(RouteGroupBuilder tags)
    {
        tags.MapGet("/", Tags.List)
            .WithName("listTags")
            .WithSummary("List resource tags");

        tags.MapPost("/", Tags.Create)
            .WithName("createTag")
            .WithSummary("Create a resource tag")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status409Conflict);

        tags.MapPatch("{id:guid}", Tags.Patch)
            .WithName("patchTag")
            .WithSummary("Update a resource tag")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status409Conflict);

        tags.MapDelete("{id:guid}", Tags.Delete)
            .WithName("deleteTag")
            .WithSummary("Delete a resource tag")
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status404NotFound);
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
            .ProduceCookie(Constants.MfaChallenge)
            .ProduceCookie(Constants.MfaSetup)
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status429TooManyRequests)
            .RequireRateLimiting("strict-auth")
            .WithName("login");

        auth.MapPost("mfa/verify", Mfa.VerifyAuthenticationMfa)
            .WithSummary("Verify an MFA login challenge")
            .WithCookie(Constants.MfaChallenge, "MFA Challenge", true)
            .ProduceCookie(Constants.RefreshToken)
            .Produces<MfaVerificationView>()
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status429TooManyRequests)
            .RequireRateLimiting("strict-auth")
            .WithName("verifyAuthenticationMfa");

        auth.MapGet("mfa/setup", Mfa.GetAuthenticationMfaSetup)
            .WithSummary("Get mandatory MFA setup")
            .WithCookie(Constants.MfaSetup, "MFA Setup", true)
            .Produces<MandatoryMfaSetupView>()
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status429TooManyRequests)
            .RequireRateLimiting("strict-auth")
            .WithName("getAuthenticationMfaSetup");

        auth.MapPost("mfa/setup/confirm", Mfa.ConfirmAuthenticationMfaSetup)
            .WithSummary("Confirm mandatory MFA setup")
            .WithCookie(Constants.MfaSetup, "MFA Setup", true)
            .ProduceCookie(Constants.RefreshToken)
            .Produces<MandatoryMfaSetupCompleteView>()
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status429TooManyRequests)
            .RequireRateLimiting("strict-auth")
            .WithName("confirmAuthenticationMfaSetup");

        auth.MapPost("logout", Authentication.Logout)
            .WithSummary("Log out")
            .WithCookie(Constants.RefreshToken, "Refresh Token", true)
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status404NotFound)
            .WithName("logout");

        auth.MapGet("oidc/providers", Oidc.ListLoginProviders)
            .WithSummary("List enabled OIDC login providers")
            .ProducesProblem(StatusCodes.Status500InternalServerError)
            .WithName("listOidcLoginProviders");

        auth.MapGet("oidc/{id:guid}/login", Oidc.BeginLogin)
            .WithSummary("Start an OIDC login flow")
            .ProducesProblem(StatusCodes.Status400BadRequest)
            .ProducesProblem(StatusCodes.Status404NotFound)
            .Produces(StatusCodes.Status302Found)
            .RequireRateLimiting("strict-auth")
            .WithName("beginOidcLogin");

        auth.MapGet("oidc/{id:guid}/callback", Oidc.CompleteLogin)
            .WithSummary("Complete an OIDC login flow")
            .ProduceCookie(Constants.RefreshToken)
            .ProducesProblem(StatusCodes.Status400BadRequest)
            .ProducesProblem(StatusCodes.Status404NotFound)
            .Produces(StatusCodes.Status302Found)
            .RequireRateLimiting("strict-auth")
            .WithName("completeOidcLogin");
    }

    private static void MapOidcProviderEndpoints(RouteGroupBuilder oidcProviders)
    {
        oidcProviders.MapGet("/", Oidc.ListProviders)
            .WithSummary("List OIDC providers")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .WithName("listOidcProviders");

        oidcProviders.MapGet("{id:guid}", Oidc.GetProvider)
            .WithSummary("Get OIDC provider")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status404NotFound)
            .WithName("getOidcProvider");

        oidcProviders.MapPost("/", Oidc.CreateProvider)
            .WithSummary("Create OIDC provider")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status409Conflict)
            .WithName("createOidcProvider");

        oidcProviders.MapPost("rename", Oidc.RenameProvider)
            .WithSummary("Rename OIDC provider")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status409Conflict)
            .WithName("renameOidcProvider");

        oidcProviders.MapPatch("{id:guid}", Oidc.UpdateProvider)
            .WithSummary("Update OIDC provider")
            .Accepts<UpdateOidcProviderInput>("application/merge-patch+json", "application/json")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status409Conflict)
            .WithName("updateOidcProvider");

        oidcProviders.MapPatch("{id:guid}/_metadata", Oidc.UpdateProviderMetadata)
            .WithSummary("Update OIDC provider metadata")
            .Accepts<PatchResourceMetadata>("application/merge-patch+json", "application/json")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status409Conflict)
            .WithName("updateOidcProviderMetadata");

        oidcProviders.MapDelete("{id:guid}", Oidc.DeleteProvider)
            .WithSummary("Delete OIDC provider")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status404NotFound)
            .WithName("deleteOidcProvider");

        oidcProviders.MapPost("{id:guid}/testDiscovery", (
                IMediator mediator,
                [FromRoute] Guid id,
                CancellationToken cancellationToken)
                => Oidc.TestDiscovery(mediator, new TestOidcProviderDiscoveryInput(id, null), cancellationToken))
            .WithSummary("Test OIDC provider discovery")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status404NotFound)
            .WithName("testOidcProviderDiscovery");

        oidcProviders.MapPost("testDiscovery", Oidc.TestDiscovery)
            .WithSummary("Test OIDC discovery")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .WithName("testOidcDiscovery");
    }

    private static void MapAutomationActionEndpoints(RouteGroupBuilder automationActions)
    {
        automationActions.MapGet("/", AutomationActions.List)
            .WithSummary("List automation actions")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .WithName("listAutomationActions");

        automationActions.MapGet("{id:guid}", AutomationActions.Get)
            .WithSummary("Get automation action")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status404NotFound)
            .WithName("getAutomationAction");

        automationActions.MapGet("{id:guid}/tags", Tags.GetAutomationActionTags)
            .WithSummary("Get automation action tags")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status404NotFound)
            .WithName("getAutomationActionTags");

        automationActions.MapPut("{id:guid}/tags", Tags.ReplaceAutomationActionTags)
            .WithSummary("Replace automation action tags")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status400BadRequest)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status404NotFound)
            .WithName("replaceAutomationActionTags");

        automationActions.MapPost("/", AutomationActions.Create)
            .WithSummary("Create automation action")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status409Conflict)
            .WithName("createAutomationAction");

        automationActions.MapPost("rename", AutomationActions.Rename)
            .WithSummary("Rename automation action")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status409Conflict)
            .WithName("renameAutomationAction");

        automationActions.MapPatch("{id:guid}", AutomationActions.Update)
            .WithSummary("Update automation action")
            .Accepts<UpdateAutomationActionInput>("application/merge-patch+json", "application/json")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status409Conflict)
            .WithName("updateAutomationAction");

        automationActions.MapPatch("{id:guid}/_metadata", AutomationActions.UpdateMetadata)
            .WithSummary("Update automation action metadata")
            .Accepts<PatchResourceMetadata>("application/merge-patch+json", "application/json")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status404NotFound)
            .WithName("updateAutomationActionMetadata");

        automationActions.MapDelete("{id:guid}", AutomationActions.Delete)
            .WithSummary("Delete automation action")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status404NotFound)
            .WithName("deleteAutomationAction");

        automationActions.MapPost("{id:guid}/run", AutomationActions.Run)
            .WithSummary("Queue automation action run")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status409Conflict)
            .WithName("runAutomationAction");

        automationActions.MapPost("{id:guid}/test", AutomationActions.Test)
            .WithSummary("Queue automation action test run")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status409Conflict)
            .WithName("testAutomationAction");

        automationActions.MapPost("{id:guid}/runs/{runId:guid}/cancel", AutomationActions.Cancel)
            .WithSummary("Cancel automation action run")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status409Conflict)
            .WithName("cancelAutomationActionRun");

        automationActions.MapGet("{id:guid}/runs", AutomationActions.ListRuns)
            .WithSummary("List automation action runs")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status404NotFound)
            .WithName("listAutomationActionRuns");

        automationActions.MapGet("{id:guid}/runs/{runId:guid}", AutomationActions.GetRun)
            .WithSummary("Get automation action run")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status404NotFound)
            .WithName("getAutomationActionRun");

        automationActions.MapGet("{id:guid}/runs/{runId:guid}/logs", AutomationActions.GetRunLogs)
            .WithSummary("Get automation action run logs")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status404NotFound)
            .WithName("getAutomationActionRunLogs");

    }

    private static void MapBackupRepositoryEndpoints(RouteGroupBuilder backupRepositories)
    {
        backupRepositories.MapGet("/", BackupRepositories.List)
            .WithSummary("List backup repositories")
            .Produces<BackupRepositoriesView>()
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .WithName("listBackupRepositories");

        backupRepositories.MapGet("{id:guid}", BackupRepositories.Get)
            .WithSummary("Get backup repository")
            .Produces<BackupRepositoryView>()
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status404NotFound)
            .WithName("getBackupRepository");

        backupRepositories.MapPost("/", BackupRepositories.Create)
            .WithSummary("Create backup repository")
            .Produces<BackupRepositoryView>()
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status409Conflict)
            .WithName("createBackupRepository");

        backupRepositories.MapPatch("{id:guid}", BackupRepositories.Update)
            .WithSummary("Update backup repository")
            .Accepts<UpdateBackupRepositoryInput>("application/merge-patch+json", "application/json")
            .Produces<BackupRepositoryView>()
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status409Conflict)
            .WithName("updateBackupRepository");

        backupRepositories.MapDelete("{id:guid}", BackupRepositories.Archive)
            .WithSummary("Archive backup repository")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status409Conflict)
            .WithName("archiveBackupRepository");

        backupRepositories.MapPost("{id:guid}/validate", BackupRepositories.Validate)
            .WithSummary("Validate backup repository")
            .Produces<BackupRepositoryValidationView>()
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status404NotFound)
            .WithName("validateBackupRepository");

        backupRepositories.MapPost("{id:guid}/initialize", BackupRepositories.Initialize)
            .WithSummary("Initialize backup repository")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status404NotFound)
            .WithName("initializeBackupRepository");

        backupRepositories.MapPost("{id:guid}/check", BackupRepositories.Check)
            .WithSummary("Check backup repository")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status404NotFound)
            .WithName("checkBackupRepository");

        backupRepositories.MapPost("{id:guid}/prune", BackupRepositories.Prune)
            .WithSummary("Prune backup repository")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status404NotFound)
            .WithName("pruneBackupRepository");
    }

    private static void MapBackupPolicyEndpoints(RouteGroupBuilder backupPolicies)
    {
        backupPolicies.MapGet("/", BackupPolicies.List)
            .WithSummary("List backup policies")
            .Produces<BackupPoliciesView>()
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .WithName("listBackupPolicies");

        backupPolicies.MapGet("platform-summaries", BackupPolicies.GetPlatformSummaries)
            .WithSummary("Get backup policy summaries for platforms")
            .Produces<PlatformBackupSummariesView>()
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .WithName("getPlatformBackupSummaries");

        backupPolicies.MapGet("{id:guid}", BackupPolicies.Get)
            .WithSummary("Get backup policy")
            .Produces<BackupPolicyView>()
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status404NotFound)
            .WithName("getBackupPolicy");

        backupPolicies.MapGet("{id:guid}/tags", Tags.GetBackupPolicyTags)
            .WithSummary("Get backup policy tags")
            .Produces<ResourceTagsView>()
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status404NotFound)
            .WithName("getBackupPolicyTags");

        backupPolicies.MapPut("{id:guid}/tags", Tags.ReplaceBackupPolicyTags)
            .WithSummary("Replace backup policy tags")
            .Produces<ResourceTagsView>()
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status400BadRequest)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status404NotFound)
            .WithName("replaceBackupPolicyTags");

        backupPolicies.MapPost("/", BackupPolicies.Create)
            .WithSummary("Create backup policy")
            .Produces<BackupPolicyView>()
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status409Conflict)
            .WithName("createBackupPolicy");

        backupPolicies.MapPatch("{id:guid}", BackupPolicies.Update)
            .WithSummary("Update backup policy")
            .Accepts<UpdateBackupPolicyInput>("application/merge-patch+json", "application/json")
            .Produces<BackupPolicyView>()
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status409Conflict)
            .WithName("updateBackupPolicy");

        backupPolicies.MapPost("rename", BackupPolicies.Rename)
            .WithSummary("Rename backup policy")
            .Produces<BackupPolicyView>()
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status409Conflict)
            .WithName("renameBackupPolicy");

        backupPolicies.MapPatch("{id:guid}/_metadata", BackupPolicies.PatchMetadata)
            .WithSummary("Update backup policy metadata")
            .Accepts<PatchResourceMetadata>("application/merge-patch+json", "application/json")
            .Produces<BackupPolicyView>()
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status409Conflict)
            .WithName("updateBackupPolicyMetadata");

        backupPolicies.MapDelete("{id:guid}", BackupPolicies.Archive)
            .WithSummary("Archive backup policy")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status409Conflict)
            .WithName("archiveBackupPolicy");

        backupPolicies.MapPost("{id:guid}/runs", BackupPolicies.QueueRun)
            .WithSummary("Queue backup policy run")
            .Produces<BackupRunView>()
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status409Conflict)
            .WithName("queueBackupRun");

        backupPolicies.MapPost("{id:guid}/run", BackupPolicies.Run)
            .WithSummary("Run backup policy and stream execution logs in real time")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status409Conflict)
            .WithName("runBackupPolicy");
    }

    private static void MapBackupRunEndpoints(RouteGroupBuilder backupRuns)
    {
        backupRuns.MapGet("/", BackupRuns.List)
            .WithSummary("List backup runs")
            .Produces<BackupRunsView>()
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .WithName("listBackupRuns");

        backupRuns.MapGet("{id:guid}", BackupRuns.Get)
            .WithSummary("Get backup run")
            .Produces<BackupRunView>()
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status404NotFound)
            .WithName("getBackupRun");

        backupRuns.MapGet("{id:guid}/logs", BackupRuns.GetLogs)
            .WithSummary("Get backup run logs")
            .Produces<BackupLogsView>()
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status404NotFound)
            .WithName("getBackupRunLogs");

        backupRuns.MapGet("{id:guid}/events", BackupRuns.GetEvents)
            .WithSummary("Get backup run events")
            .Produces<BackupEventsView>()
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status404NotFound)
            .WithName("getBackupRunEvents");

        backupRuns.MapPost("{id:guid}/cancel", BackupRuns.Cancel)
            .WithSummary("Cancel backup run")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status404NotFound)
            .WithName("cancelBackupRun");

        backupRuns.MapPost("{id:guid}/restoreVolume", BackupRuns.RestoreVolume)
            .WithSummary("Queue backup volume restore")
            .Produces<BackupRestoreRunView>()
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status404NotFound)
            .WithName("restoreBackupVolume");

        backupRuns.MapPost("{id:guid}/restoreVolume/run", BackupRuns.RunRestoreVolume)
            .WithSummary("Run backup volume restore")
            .Produces<BackupRestoreRunStreamItem[]>()
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status404NotFound)
            .WithName("runBackupRestoreVolume");
    }

    private static void MapBackupRestoreRunEndpoints(RouteGroupBuilder backupRestoreRuns)
    {
        backupRestoreRuns.MapGet("/", BackupRestoreRuns.List)
            .WithSummary("List backup restore runs")
            .Produces<BackupRestoreRunsView>()
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .WithName("listBackupRestoreRuns");

        backupRestoreRuns.MapGet("{id:guid}", BackupRestoreRuns.Get)
            .WithSummary("Get backup restore run")
            .Produces<BackupRestoreRunView>()
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status404NotFound)
            .WithName("getBackupRestoreRun");

        backupRestoreRuns.MapGet("{id:guid}/logs", BackupRestoreRuns.GetLogs)
            .WithSummary("Get backup restore run logs")
            .Produces<BackupLogsView>()
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status404NotFound)
            .WithName("getBackupRestoreRunLogs");

        backupRestoreRuns.MapGet("{id:guid}/events", BackupRestoreRuns.GetEvents)
            .WithSummary("Get backup restore run events")
            .Produces<BackupEventsView>()
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status404NotFound)
            .WithName("getBackupRestoreRunEvents");

        backupRestoreRuns.MapPost("{id:guid}/cancel", BackupRestoreRuns.Cancel)
            .WithSummary("Cancel backup restore run")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status404NotFound)
            .WithName("cancelBackupRestoreRun");
    }

    private static void MapBuildProjectEndpoints(RouteGroupBuilder buildProjects)
    {
        buildProjects.MapGet("/", Builds.List)
            .WithSummary("List build projects")
            .Produces<BuildProjectsView>()
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .WithName("listBuildProjects");

        buildProjects.MapGet("{id:guid}", Builds.Get)
            .WithSummary("Get build project")
            .Produces<BuildProjectView>()
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status404NotFound)
            .WithName("getBuildProject");

        buildProjects.MapGet("{id:guid}/tags", Tags.GetBuildTags)
            .WithSummary("Get build project tags")
            .Produces<ResourceTagsView>()
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status404NotFound)
            .WithName("getBuildTags");

        buildProjects.MapPut("{id:guid}/tags", Tags.ReplaceBuildTags)
            .WithSummary("Replace build project tags")
            .Produces<ResourceTagsView>()
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status400BadRequest)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status404NotFound)
            .WithName("replaceBuildTags");

        buildProjects.MapPost("/", Builds.Create)
            .WithSummary("Create build project")
            .Produces<BuildProjectView>()
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status409Conflict)
            .WithName("createBuildProject");

        buildProjects.MapPatch("{id:guid}", Builds.Update)
            .WithSummary("Update build project")
            .Accepts<UpdateBuildProjectInput>("application/merge-patch+json", "application/json")
            .Produces<BuildProjectView>()
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status409Conflict)
            .WithName("updateBuildProject");

        buildProjects.MapPost("rename", Builds.Rename)
            .WithSummary("Rename build project")
            .Produces<BuildProjectView>()
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status409Conflict)
            .WithName("renameBuild");

        buildProjects.MapPatch("{id:guid}/_metadata", Builds.PatchMetadata)
            .WithSummary("Update build project metadata")
            .Accepts<PatchResourceMetadata>("application/merge-patch+json", "application/json")
            .Produces<BuildProjectView>()
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status409Conflict)
            .WithName("updateBuildMetadata");

        buildProjects.MapDelete("{id:guid}", Builds.Archive)
            .WithSummary("Archive build project")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status409Conflict)
            .WithName("archiveBuildProject");

        buildProjects.MapPost("{id:guid}/runs", Builds.QueueRun)
            .WithSummary("Queue build run")
            .Produces<BuildRunView>()
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status409Conflict)
            .WithName("queueBuildRun");
    }

    private static void MapBuildRunEndpoints(RouteGroupBuilder buildRuns)
    {
        buildRuns.MapGet("/", BuildRuns.List)
            .WithSummary("List build runs")
            .Produces<BuildRunsView>()
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .WithName("listBuildRuns");

        buildRuns.MapGet("{id:guid}", BuildRuns.Get)
            .WithSummary("Get build run")
            .Produces<BuildRunView>()
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status404NotFound)
            .WithName("getBuildRun");

        buildRuns.MapGet("{id:guid}/logs", BuildRuns.GetLogs)
            .WithSummary("Get build run logs")
            .Produces<BuildLogsView>()
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status404NotFound)
            .WithName("getBuildRunLogs");

        buildRuns.MapPost("{id:guid}/cancel", BuildRuns.Cancel)
            .WithSummary("Cancel build run")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status404NotFound)
            .WithName("cancelBuildRun");
    }

    private static void MapBuildAgentPoolEndpoints(RouteGroupBuilder buildAgentPools)
    {
        buildAgentPools.MapGet("/", BuildAgentPools.List)
            .WithSummary("List build pools")
            .Produces<BuildAgentPoolsView>()
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .WithName("listBuildAgentPools");

        buildAgentPools.MapGet("{id:guid}", BuildAgentPools.Get)
            .WithSummary("Get build pool")
            .Produces<BuildAgentPoolView>()
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status404NotFound)
            .WithName("getBuildAgentPool");

        buildAgentPools.MapGet("{id:guid}/tags", Tags.GetBuildAgentPoolTags)
            .WithSummary("Get build pool tags")
            .Produces<ResourceTagsView>()
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status404NotFound)
            .WithName("getBuildAgentPoolTags");

        buildAgentPools.MapPut("{id:guid}/tags", Tags.ReplaceBuildAgentPoolTags)
            .WithSummary("Replace build pool tags")
            .Produces<ResourceTagsView>()
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status400BadRequest)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status404NotFound)
            .WithName("replaceBuildAgentPoolTags");

        buildAgentPools.MapPost("/", BuildAgentPools.Create)
            .WithSummary("Create build pool")
            .Produces<BuildAgentPoolView>()
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status409Conflict)
            .WithName("createBuildAgentPool");

        buildAgentPools.MapPatch("{id:guid}", BuildAgentPools.Update)
            .WithSummary("Update build pool")
            .Accepts<UpdateBuildAgentPoolInput>("application/merge-patch+json", "application/json")
            .Produces<BuildAgentPoolView>()
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status409Conflict)
            .WithName("updateBuildAgentPool");

        buildAgentPools.MapPost("rename", BuildAgentPools.Rename)
            .WithSummary("Rename build pool")
            .Produces<BuildAgentPoolView>()
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status409Conflict)
            .WithName("renameBuildAgentPool");

        buildAgentPools.MapPatch("{id:guid}/_metadata", BuildAgentPools.PatchMetadata)
            .WithSummary("Update build pool metadata")
            .Accepts<PatchResourceMetadata>("application/merge-patch+json", "application/json")
            .Produces<BuildAgentPoolView>()
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status409Conflict)
            .WithName("updateBuildAgentPoolMetadata");

        buildAgentPools.MapPost("{id:guid}/test", BuildAgentPools.Test)
            .WithSummary("Test build pool")
            .Produces<BuildAgentPoolView>()
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status409Conflict)
            .WithName("testBuildAgentPool");

        buildAgentPools.MapPost("{id:guid}/edge/enrollments", BuildAgentPools.CreateEdgeEnrollment)
            .WithSummary("Create build pool Edge Agent enrollment")
            .Produces<EdgeAgentEnrollmentView>()
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status400BadRequest)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status409Conflict)
            .WithName("createBuildAgentPoolEdgeEnrollment");

        buildAgentPools.MapGet("{id:guid}/edge/status", BuildAgentPools.GetEdgeStatus)
            .WithSummary("Get build pool Edge Agent status")
            .Produces<EdgeAgentStatusView>()
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status400BadRequest)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status404NotFound)
            .WithName("getBuildAgentPoolEdgeStatus");

        buildAgentPools.MapPost("{id:guid}/edge/revoke", BuildAgentPools.RevokeEdge)
            .WithSummary("Revoke build pool Edge Agent")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status400BadRequest)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status404NotFound)
            .WithName("revokeBuildAgentPoolEdgeAgent");

        buildAgentPools.MapDelete("{id:guid}", BuildAgentPools.Archive)
            .WithSummary("Archive build pool")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status409Conflict)
            .WithName("archiveBuildAgentPool");
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

        containers.MapGet("{id}/data", Containers.GetContainerData)
            .WithSummary("Get container data")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("getContainerData");

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

        containers.MapGet("{id:guid}/adoption-draft", Containers.GetAdoptionDraft)
            .WithSummary("Build an adoption draft for an unmanaged container")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status409Conflict)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("getContainerAdoptionDraft");

        containers.MapPost("{id:guid}/adopt", Containers.Adopt)
            .WithSummary("Adopt an unmanaged container as a deployment")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status409Conflict)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("adoptContainer");

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
        platforms.MapGet("{platformId:guid}/swarm", SwarmInventory.GetOverview)
            .WithSummary("Get Docker Swarm cluster health and persisted inventory")
            .ProducesProblem(StatusCodes.Status400BadRequest)
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("getSwarmOverview");

        platforms.MapGet("{platformId:guid}/swarm/nodes", SwarmNodes.List)
            .WithSummary("List the persisted nodes observed on a Docker Swarm platform")
            .ProducesProblem(StatusCodes.Status400BadRequest)
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("listSwarmNodes");

        platforms.MapGet("{platformId:guid}/swarm/nodes/{nodeId}", SwarmNodes.Get)
            .WithSummary("Get a persisted Docker Swarm node")
            .ProducesProblem(StatusCodes.Status400BadRequest)
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("getSwarmNode");

        platforms.MapGet("{platformId:guid}/swarm/nodes/{nodeId}/inspect", SwarmNodes.Inspect)
            .WithSummary("Inspect a live Docker Swarm node")
            .ProducesValidationProblem().ProducesProblem(StatusCodes.Status400BadRequest)
            .ProducesProblem(StatusCodes.Status404NotFound).ProducesProblem(StatusCodes.Status409Conflict)
            .ProducesProblem(StatusCodes.Status403Forbidden).ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("inspectSwarmNode");

        platforms.MapGet("{platformId:guid}/swarm/services", SwarmInventory.ListServices)
            .WithSummary("List the persisted services observed on a Docker Swarm platform")
            .ProducesProblem(StatusCodes.Status400BadRequest).ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status403Forbidden).ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("listSwarmServices");
        platforms.MapGet("{platformId:guid}/swarm/services/{resourceId}", SwarmInventory.GetService)
            .WithSummary("Get a persisted Docker Swarm service")
            .ProducesProblem(StatusCodes.Status400BadRequest).ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status403Forbidden).ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("getSwarmService");
        platforms.MapGet("{platformId:guid}/swarm/services/{resourceId}/inspect", SwarmInventory.InspectService)
            .WithSummary("Inspect a live Docker Swarm service")
            .ProducesValidationProblem().ProducesProblem(StatusCodes.Status400BadRequest)
            .ProducesProblem(StatusCodes.Status404NotFound).ProducesProblem(StatusCodes.Status409Conflict)
            .ProducesProblem(StatusCodes.Status403Forbidden).ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("inspectSwarmService");
        platforms.MapGet("{platformId:guid}/swarm/services/{resourceId}/logs", SwarmInventory.GetServiceLogs)
            .WithSummary("Get a bounded tail of Docker Swarm service logs")
            .ProducesValidationProblem().ProducesProblem(StatusCodes.Status400BadRequest)
            .ProducesProblem(StatusCodes.Status404NotFound).ProducesProblem(StatusCodes.Status409Conflict)
            .ProducesProblem(StatusCodes.Status403Forbidden).ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("getSwarmServiceLogs");
        platforms.MapGet("{platformId:guid}/swarm/tasks", SwarmInventory.ListTasks)
            .WithSummary("List a bounded set of persisted Docker Swarm tasks")
            .ProducesValidationProblem().ProducesProblem(StatusCodes.Status400BadRequest)
            .ProducesProblem(StatusCodes.Status404NotFound).ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized).WithName("listSwarmTasks");
        platforms.MapGet("{platformId:guid}/swarm/tasks/{resourceId}", SwarmInventory.GetTask)
            .WithSummary("Get a persisted Docker Swarm task")
            .ProducesProblem(StatusCodes.Status400BadRequest).ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status403Forbidden).ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("getSwarmTask");
        platforms.MapGet("{platformId:guid}/swarm/tasks/{resourceId}/inspect", SwarmInventory.InspectTask)
            .WithSummary("Inspect a live Docker Swarm task")
            .ProducesValidationProblem().ProducesProblem(StatusCodes.Status400BadRequest)
            .ProducesProblem(StatusCodes.Status404NotFound).ProducesProblem(StatusCodes.Status409Conflict)
            .ProducesProblem(StatusCodes.Status403Forbidden).ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("inspectSwarmTask");
        platforms.MapGet("{platformId:guid}/swarm/tasks/{resourceId}/stats", SwarmInventory.GetTaskStats)
            .WithSummary("Get historical stats for a task running on the connected Swarm node")
            .ProducesValidationProblem().ProducesProblem(StatusCodes.Status400BadRequest)
            .ProducesProblem(StatusCodes.Status404NotFound).ProducesProblem(StatusCodes.Status409Conflict)
            .ProducesProblem(StatusCodes.Status403Forbidden).ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("getSwarmTaskStats");
        platforms.MapGet("{platformId:guid}/swarm/tasks/{resourceId}/logs", SwarmInventory.GetTaskLogs)
            .WithSummary("Get a bounded tail of Docker Swarm task logs")
            .ProducesValidationProblem().ProducesProblem(StatusCodes.Status400BadRequest)
            .ProducesProblem(StatusCodes.Status404NotFound).ProducesProblem(StatusCodes.Status409Conflict)
            .ProducesProblem(StatusCodes.Status403Forbidden).ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("getSwarmTaskLogs");
        platforms.MapGet("{platformId:guid}/swarm/networks", SwarmInventory.ListNetworks)
            .WithSummary("List the persisted cluster networks observed on a Docker Swarm platform")
            .ProducesProblem(StatusCodes.Status400BadRequest).ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status403Forbidden).ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("listSwarmNetworks");
        platforms.MapGet("{platformId:guid}/swarm/networks/{resourceId}", SwarmInventory.GetNetwork)
            .WithSummary("Get a persisted Docker Swarm network")
            .ProducesProblem(StatusCodes.Status400BadRequest).ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status403Forbidden).ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("getSwarmNetwork");
        platforms.MapGet("{platformId:guid}/swarm/secrets", SwarmInventory.ListSecrets)
            .WithSummary("List Docker Swarm secret metadata without secret data")
            .ProducesProblem(StatusCodes.Status400BadRequest).ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status403Forbidden).ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("listSwarmSecrets");
        platforms.MapGet("{platformId:guid}/swarm/secrets/{resourceId}", SwarmInventory.GetSecret)
            .WithSummary("Get Docker Swarm secret metadata without secret data")
            .ProducesProblem(StatusCodes.Status400BadRequest).ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status403Forbidden).ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("getSwarmSecret");
        platforms.MapGet("{platformId:guid}/swarm/configs", SwarmInventory.ListConfigs)
            .WithSummary("List persisted Docker Swarm config metadata")
            .ProducesProblem(StatusCodes.Status400BadRequest).ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status403Forbidden).ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("listSwarmConfigs");
        platforms.MapGet("{platformId:guid}/swarm/configs/{resourceId}", SwarmInventory.GetConfig)
            .WithSummary("Get persisted Docker Swarm config metadata")
            .ProducesProblem(StatusCodes.Status400BadRequest).ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status403Forbidden).ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("getSwarmConfig");

        platforms.MapGet("{platformId:guid}/unmanaged-compose-projects/{projectName}", Stacks.GetComposeImportDraft)
            .WithSummary("Get an unmanaged Docker Compose project import draft")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status409Conflict)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("getComposeProjectImportDraft");

        platforms.MapPost("{platformId:guid}/unmanaged-compose-projects/{projectName}/import-draft", Stacks.ValidateComposeImportDraft)
            .WithSummary("Validate a source for an unmanaged Docker Compose project")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status409Conflict)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("validateComposeProjectImportDraft");

        platforms.MapPost("{platformId:guid}/unmanaged-compose-projects/{projectName}/import", Stacks.ImportComposeProject)
            .WithSummary("Import an unmanaged Docker Compose project as a stack")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status409Conflict)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("importComposeProject");

        platforms.MapGet("/", Platforms.List)
            .WithSummary("List all platforms")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("listPlatforms");

        platforms.MapGet("agent/setup", Platforms.GetAgentSetup)
            .WithSummary("Get regular Agent setup instructions")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("getAgentSetup");

        platforms.MapPost("agent/setup/rotate-key", Platforms.RotateAgentHubKey)
            .WithSummary("Rotate regular Agent hub key pair")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("rotateAgentHubKey");

        platforms.MapGet("{id}", Platforms.Get)
            .WithSummary("Get platform by Id")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("getPlatfom");

        platforms.MapGet("{id}/tags", Tags.GetPlatformTags)
            .WithSummary("Get platform tags")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("getPlatformTags");

        platforms.MapPut("{id}/tags", Tags.ReplacePlatformTags)
            .WithSummary("Replace platform tags")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status400BadRequest)
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("replacePlatformTags");

        platforms.MapGet("{id}/containers", Platforms.ListContainers)
            .WithSummary("Returns the list of containers of the given platform")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("listContainers");

        platforms.MapGet("{platformId}/volumes/{name}/files", Volumes.ListDirectory)
            .WithSummary("List volume directory contents")
            .Produces<VolumeDirectoryView>()
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status400BadRequest)
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("listVolumeDirectory");

        platforms.MapGet("{platformId}/volumes/{name}/files/download", Volumes.Download)
            .WithSummary("Download a volume file or directory archive")
            .Produces(StatusCodes.Status200OK, contentType: "application/octet-stream")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status400BadRequest)
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("downloadVolumePath");

        platforms.MapGet("{id}/stats", Platforms.GetStats)
            .WithSummary("Get platform stats")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("getPlatformStats");

        platforms.MapPost("{id}/prune", Platforms.Prune)
            .WithSummary("Delete unused Docker resources on a platform")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status400BadRequest)
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("prunePlatform");

        platforms.MapPost("{id}/edge/enrollments", Platforms.CreateEdgeEnrollment)
            .WithSummary("Create an Edge Agent enrollment token for a platform")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status400BadRequest)
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status409Conflict)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("createEdgeAgentEnrollment");

        platforms.MapGet("{id}/edge/status", Platforms.GetEdgeStatus)
            .WithSummary("Get Edge Agent connection status for a platform")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status400BadRequest)
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("getEdgeAgentStatus");

        platforms.MapPost("{id}/edge/revoke", Platforms.RevokeEdge)
            .WithSummary("Revoke an Edge Agent binding for a platform")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status400BadRequest)
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("revokeEdgeAgent");

        platforms.MapPost("rename", PlatformMetadata.Rename)
            .WithSummary("Rename a platform")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status409Conflict)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("renamePlatform");

        platforms.MapPatch("{id}", Platforms.Patch)
            .WithSummary("Patch a platform")
            .Accepts<PlatformInput>("application/merge-patch+json", "application/json")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status409Conflict)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("updatePlatform");

        platforms.MapPatch("{id}/_metadata", PlatformMetadata.PatchMetadata)
            .WithSummary("Patch platform metadata")
            .Accepts<PatchResourceMetadata>("application/merge-patch+json", "application/json")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("updatePlatformMetadata");

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

        registries.MapGet("{id}/tags", Tags.GetRegistryTags)
            .WithSummary("Get registry tags")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("getRegistryTags");

        registries.MapPut("{id}/tags", Tags.ReplaceRegistryTags)
            .WithSummary("Replace registry tags")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("replaceRegistryTags");

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

        deployment.MapPost("/{deploymentId:guid}/check-updates", Deployments.CheckUpdates)
            .WithSummary("Check a deployment image for updates")
            .Produces<DeploymentView>()
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status400BadRequest)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status409Conflict)
            .ProducesProblem(StatusCodes.Status502BadGateway)
            .WithName("checkDeploymentUpdates");

        deployment.MapGet("/{deploymentId}/tags", Tags.GetDeploymentTags)
            .WithSummary("Get deployment tags")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status404NotFound)
            .WithName("getDeploymentTags");

        deployment.MapPut("/{deploymentId}/tags", Tags.ReplaceDeploymentTags)
            .WithSummary("Replace deployment tags")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status400BadRequest)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status404NotFound)
            .WithName("replaceDeploymentTags");

        deployment.MapGet("{id}/stats", Deployments.GetStats)
            .WithSummary("Get deployment stats")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("getDeploymentStats");

        deployment.MapGet("/{deploymentId}/_cfg", Deployments.GetConfig)
            .WithSummary("Get deployment configuration")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status404NotFound)
            .WithName("getDeploymentConfig");

        deployment.MapGet("/{deploymentId}/duplicate-draft", Deployments.GetDuplicateDraft)
            .WithSummary("Get deployment duplicate draft")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status404NotFound)
            .WithName("getDeploymentDuplicateDraft");

        deployment.MapGet("/{deploymentId}/backup-source-preview", Deployments.GetBackupSourcePreview)
            .WithSummary("Preview deployment backup source volumes")
            .Produces<DeploymentBackupSourcePreviewView>()
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status404NotFound)
            .WithName("getDeploymentBackupSourcePreview");

        deployment.MapGet("{id}/info", Deployments.GetInfo)
            .WithSummary("Get basic container details")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("getDeploymentContainerInfo");

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

        deployment.MapGet("{id}/inspect", Deployments.Inspect)
            .WithSummary("Inspect a deployment")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("inspectDeployment");
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

        stacks.MapPost("/{stackId:guid}/check-updates", Stacks.CheckUpdates)
            .WithSummary("Check a stack source for updates")
            .Produces<StackView>()
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status400BadRequest)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status409Conflict)
            .ProducesProblem(StatusCodes.Status502BadGateway)
            .WithName("checkStackUpdates");

        stacks.MapGet("/{stackId}/tags", Tags.GetStackTags)
            .WithSummary("Get stack tags")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status404NotFound)
            .WithName("getStackTags");

        stacks.MapPut("/{stackId}/tags", Tags.ReplaceStackTags)
            .WithSummary("Replace stack tags")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status400BadRequest)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status404NotFound)
            .WithName("replaceStackTags");

        stacks.MapGet("/{stackId}/_cfg", Stacks.GetConfig)
            .WithSummary("Get stack configuration")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status404NotFound)
            .WithName("getStackConfig");

        stacks.MapGet("/{stackId}/backup-source-preview", Stacks.GetBackupSourcePreview)
            .WithSummary("Preview stack backup source volumes")
            .Produces<StackBackupSourcePreviewView>()
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status404NotFound)
            .WithName("getStackBackupSourcePreview");

        stacks.MapGet("/{stackId}/duplicate-draft", Stacks.GetDuplicateDraft)
            .WithSummary("Get stack duplicate draft")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status404NotFound)
            .WithName("getStackDuplicateDraft");

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

        stacks.MapPost("/apply", Stacks.ApplyStack)
            .WithSummary("Apply a stack and streams execution logs in real time.")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("applyStack");

        stacks.MapPost("/rollback", Stacks.RollbackStack)
            .WithSummary("Rollback a stack to a previous release and stream execution logs in real time.")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("rollbackStack");

        stacks.MapPost("/stop", Stacks.Stop)
            .WithSummary("Stop stacks")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("stopStacks");

        stacks.MapPost("/start", Stacks.Start)
            .WithSummary("Start stacks")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("startStacks");

        stacks.MapPost("/pause", Stacks.Pause)
            .WithSummary("Pause stacks")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("pauseStacks");

        stacks.MapPost("/resume", Stacks.Resume)
            .WithSummary("Resume stacks")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("resumeStacks");

        stacks.MapPost("/restart", Stacks.Restart)
            .WithSummary("Restart stacks")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("restartStacks");

        stacks.MapGet("{stackId}/data", Stacks.GetContainersData)
            .WithSummary("Get containers data")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("getContainersData");

        stacks.MapGet("{stackId}/stats", Stacks.GetStats)
            .WithSummary("Get stack stats")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("getStackStats");

        stacks.MapGet("{stackId}/drift", Stacks.GetDrift)
            .WithSummary("Get stack drift report")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("getStackDrift");

        stacks.MapPut("{stackId}/drift-policy", Stacks.UpdateDriftPolicy)
            .WithSummary("Update stack drift policy")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status400BadRequest)
            .WithName("updateStackDriftPolicy");

        stacks.MapPost("{stackId}/reconcile", Stacks.Reconcile)
            .WithSummary("Reconcile safe stack drift")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .ProducesProblem(StatusCodes.Status400BadRequest)
            .WithName("reconcileStack");

        stacks.MapGet("{stackId}/containers/{containerId}/inspect", Stacks.InspectContainer)
            .WithSummary("Inspect a stack container")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("inspectStackContainer");
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

    private static void MapLookupEndpoints(RouteGroupBuilder lookup)
    {
        lookup.MapGet("/", Lookup.Get)
            .WithSummary("Lookup resources")
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status400BadRequest)
            .ProducesProblem(StatusCodes.Status404NotFound)
            .ProducesProblem(StatusCodes.Status403Forbidden)
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("lookup");
    }

    private static void MapSearchEndpoints(RouteGroupBuilder search)
    {
        search.MapGet("/", GlobalSearch.Get)
            .WithSummary("Search accessible resources")
            .Produces<GlobalSearchResponse>()
            .ProducesValidationProblem()
            .ProducesProblem(StatusCodes.Status401Unauthorized)
            .WithName("globalSearch");
    }
}
