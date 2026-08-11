using Application.Services.Abstractions;
using Application.Services.SignalR;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities;
using Domain.Entities.Alerts;
using Domain.Entities.Identity;
using Domain.Entities.Platforms;
using Domain.Entities.SwarmServices;
using Hosting.Common;
using Microsoft.AspNetCore.Http;
using Microsoft.AspNetCore.SignalR;
using Microsoft.AspNetCore.SignalR.Client;
using Microsoft.Extensions.DependencyInjection;
using System.IdentityModel.Tokens.Jwt;
using System.Security.Claims;
using Tests.Integration.Helpers;
using WebApi.Hubs;
using WebApi.Routes.Endpoints.Resources.Alerters;

namespace Tests.Integration.WebApi.Hubs;

public abstract class ApplicationHubAuthorizationTestBase(PostgresTestFixture fixture)
    : IntegrationTestBase(fixture)
{
    private static readonly Guid AdminRoleId = Guid.Parse("30000000-0000-0000-0000-000000000001");
    private readonly List<HubConnection> connections = [];
    private Guid platformId;
    private Guid containerResourceId;
    private string containerId = string.Empty;
    private Guid swarmServiceId;

    protected override void ConfigureTestServices(IServiceCollection services)
    {
        services.PostConfigure<HubOptions>(options =>
        {
            options.AddFilter<TestHttpContextSyncFilter>();
        });
    }

    protected override async ValueTask SeedDbAsync(IUnitOfWork unitOfWork)
    {
        var platform = Fakes.GetDummyPlatform();
        var container = new Container(
            "signalr-auth-container",
            "sha256:test",
            platform.Id,
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
            ContainerStateStatus.Running);
        var swarmService = new SwarmService(
            "signalr-managed-service",
            platform.Id,
            Constants.SystemId,
            new SwarmServiceSpec
            {
                Image = new SwarmExternalImage(Guid.CreateVersion7(), "nginx:latest")
            });

        await unitOfWork.Platforms.AddAsync(platform, TestContext.Current.CancellationToken);
        await unitOfWork.Containers.AddAsync(container, TestContext.Current.CancellationToken);
        await unitOfWork.SwarmServices.AddAsync(swarmService, TestContext.Current.CancellationToken);
        await unitOfWork.CommitAsync(TestContext.Current.CancellationToken);

        platformId = platform.Id;
        containerResourceId = container.Id;
        containerId = container.DockerContainerId[..12];
        swarmServiceId = swarmService.Id;
    }

    protected async Task RunRepresentativePermissionMatrixScenarioAsync()
    {
        var resourceId = Guid.CreateVersion7();
        var admin = await CreateAuthorizationSubjectAsync(directRoleId: AdminRoleId);
        var directGlobal = await CreateAuthorizationSubjectAsync(directRoleId: ViewerRoleId);
        var teamGlobal = await CreateAuthorizationSubjectAsync(teamRoleId: ViewerRoleId);
        var directGrant = await CreateAuthorizationSubjectAsync(
            resourceGrants: [new ResourceGrant(ResourceType.Deployment, resourceId, PermissionLevel.Read)]);
        var teamGrant = await CreateTeamResourceGrantSubjectAsync(
            ResourceType.Deployment,
            resourceId,
            PermissionLevel.Read);
        var noGrant = await CreateAuthorizationSubjectAsync();
        var disabled = await CreateAuthorizationSubjectAsync(teamRoleId: ViewerRoleId);
        await SetActorEnabledAsync(disabled.ActorId, false);

        await AssertAuthorizationAllowedAsync(admin, Constants.WellKnownSignalRGroups.DeploymentGroup(resourceId));
        await AssertAuthorizationAllowedAsync(directGlobal, Constants.WellKnownSignalRGroups.DeploymentGroup(resourceId));
        await AssertAuthorizationAllowedAsync(teamGlobal, Constants.WellKnownSignalRGroups.DeploymentGroup(resourceId));
        await AssertAuthorizationAllowedAsync(directGrant, Constants.WellKnownSignalRGroups.DeploymentGroup(resourceId));
        await AssertAuthorizationAllowedAsync(teamGrant, Constants.WellKnownSignalRGroups.DeploymentGroup(resourceId));
        await AssertAuthorizationDeniedAsync(directGrant, Constants.WellKnownSignalRGroups.DeploymentsGroup);
        await AssertAuthorizationDeniedAsync(noGrant, Constants.WellKnownSignalRGroups.DeploymentGroup(resourceId));
        await AssertAuthorizationDeniedAsync(disabled, Constants.WellKnownSignalRGroups.DeploymentGroup(resourceId));
    }

    protected async Task RunSpecificPermissionScenarioAsync()
    {
        var stackId = Guid.CreateVersion7();
        var readOnly = await CreateAuthorizationSubjectAsync(
            resourceGrants:
            [
                new ResourceGrant(ResourceType.Stack, stackId, PermissionLevel.Read),
                new ResourceGrant(ResourceType.Platform, platformId, PermissionLevel.Read)
            ]);
        var logReader = await CreateAuthorizationSubjectAsync(
            resourceGrants:
            [
                new ResourceGrant(
                    ResourceType.Stack,
                    stackId,
                    PermissionLevel.Read,
                    SpecificPermission.Logs)
            ]);
        var terminalUser = await CreateAuthorizationSubjectAsync(
            resourceGrants:
            [
                new ResourceGrant(
                    ResourceType.Platform,
                    platformId,
                    PermissionLevel.Read,
                    SpecificPermission.Terminal)
            ]);
        var containerLogReader = await CreateAuthorizationSubjectAsync(
            resourceGrants:
            [
                new ResourceGrant(
                    ResourceType.Platform,
                    platformId,
                    PermissionLevel.Read,
                    SpecificPermission.Logs)
            ]);
        var noPlatformAccess = await CreateAuthorizationSubjectAsync();

        await AssertAuthorizationAllowedAsync(readOnly, Constants.WellKnownSignalRGroups.ContainerInfoGroup(containerId));
        await AssertAuthorizationAllowedAsync(
            readOnly,
            Constants.WellKnownSignalRGroups.ContainerInfoGroup(containerResourceId.ToString("D")));
        await AssertAuthorizationDeniedAsync(noPlatformAccess, Constants.WellKnownSignalRGroups.ContainerInfoGroup(containerId));
        await AssertAuthorizationDeniedAsync(readOnly, Constants.WellKnownSignalRGroups.StackLogGroup(stackId));
        await AssertAuthorizationDeniedAsync(readOnly, Constants.WellKnownSignalRGroups.ContainerLogGroup(containerId));
        await AssertAuthorizationDeniedAsync(
            readOnly,
            Constants.WellKnownSignalRGroups.ContainerExecGroup(containerId, "read-only-session"));
        await AssertAuthorizationAllowedAsync(logReader, Constants.WellKnownSignalRGroups.StackLogGroup(stackId));
        await AssertAuthorizationAllowedAsync(
            containerLogReader,
            Constants.WellKnownSignalRGroups.ContainerLogGroup(containerId));
        await AssertAuthorizationAllowedAsync(
            terminalUser,
            Constants.WellKnownSignalRGroups.ContainerExecGroup(containerId, "terminal-session"));
    }

    protected async Task RunSwarmServiceParentVisibilityScenarioAsync()
    {
        var serviceOnly = await CreateAuthorizationSubjectAsync(
            resourceGrants:
            [
                new ResourceGrant(ResourceType.SwarmService, swarmServiceId, PermissionLevel.Read)
            ]);
        var platformOnly = await CreateAuthorizationSubjectAsync(
            resourceGrants:
            [
                new ResourceGrant(ResourceType.Platform, platformId, PermissionLevel.Read)
            ]);
        var visible = await CreateAuthorizationSubjectAsync(
            resourceGrants:
            [
                new ResourceGrant(ResourceType.SwarmService, swarmServiceId, PermissionLevel.Read),
                new ResourceGrant(ResourceType.Platform, platformId, PermissionLevel.Read)
            ]);
        var globalViewer = await CreateAuthorizationSubjectAsync(directRoleId: ViewerRoleId);

        await AssertAuthorizationDeniedAsync(
            serviceOnly,
            Constants.WellKnownSignalRGroups.SwarmServiceGroup(swarmServiceId));
        await AssertAuthorizationDeniedAsync(
            platformOnly,
            Constants.WellKnownSignalRGroups.SwarmServiceGroup(swarmServiceId));
        await AssertAuthorizationAllowedAsync(
            visible,
            Constants.WellKnownSignalRGroups.SwarmServiceGroup(swarmServiceId));
        await AssertAuthorizationAllowedAsync(
            globalViewer,
            Constants.WellKnownSignalRGroups.SwarmServicesGroupForPlatform(platformId));
        await AssertAuthorizationDeniedAsync(globalViewer, Constants.WellKnownSignalRGroups.SwarmServicesGroup);
    }

    protected async Task RunProtectedGroupDeliveryScenarioAsync()
    {
        var resourceId = Guid.CreateVersion7();
        var authorized = await CreateAuthorizationSubjectAsync(
            resourceGrants: [new ResourceGrant(ResourceType.Deployment, resourceId, PermissionLevel.Read)]);
        var unauthorized = await CreateAuthorizationSubjectAsync();
        var authorizedEvent = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
        var unauthorizedEvent = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
        var unauthorizedBarrier = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
        var authorizedConnection = await CreateConnectionAsync(authorized);
        var unauthorizedConnection = await CreateConnectionAsync(unauthorized);
        authorizedConnection.On("ProtectedEvent", () => authorizedEvent.TrySetResult());
        unauthorizedConnection.On("ProtectedEvent", () => unauthorizedEvent.TrySetResult());
        unauthorizedConnection.On("DeliveryBarrier", () => unauthorizedBarrier.TrySetResult());
        var groupId = Constants.WellKnownSignalRGroups.DeploymentGroup(resourceId);

        await authorizedConnection.InvokeAsync(
            "JoinGroup",
            groupId,
            cancellationToken: TestContext.Current.CancellationToken);
        await Assert.ThrowsAsync<HubException>(() => unauthorizedConnection.InvokeAsync(
            "JoinGroup",
            groupId,
            cancellationToken: TestContext.Current.CancellationToken));

        var hubContext = Services.GetRequiredService<IHubContext<ApplicationHub>>();
        await hubContext.Clients.Group(groupId).SendAsync(
            "ProtectedEvent",
            TestContext.Current.CancellationToken);
        await hubContext.Clients.Client(unauthorizedConnection.ConnectionId!).SendAsync(
            "DeliveryBarrier",
            TestContext.Current.CancellationToken);

        await authorizedEvent.Task.WaitAsync(
            TimeSpan.FromSeconds(5),
            TestContext.Current.CancellationToken);
        await unauthorizedBarrier.Task.WaitAsync(
            TimeSpan.FromSeconds(5),
            TestContext.Current.CancellationToken);
        Assert.False(unauthorizedEvent.Task.IsCompleted);
    }

    protected async Task RunAlertEventDeliveryScenarioAsync()
    {
        var recipient = await CreateAuthorizationSubjectAsync();
        var nonRecipient = await CreateAuthorizationSubjectAsync();
        var recipientEvent = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
        var nonRecipientEvent = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
        var nonRecipientBarrier = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
        var recipientConnection = await CreateConnectionAsync(recipient);
        var nonRecipientConnection = await CreateConnectionAsync(nonRecipient);
        recipientConnection.On<AlertEventView>(
            "AlertEventReceived",
            _ => recipientEvent.TrySetResult());
        nonRecipientConnection.On<AlertEventView>(
            "AlertEventReceived",
            _ => nonRecipientEvent.TrySetResult());
        nonRecipientConnection.On("DeliveryBarrier", () => nonRecipientBarrier.TrySetResult());

        await recipientConnection.InvokeAsync(
            "JoinGroup",
            Constants.WellKnownSignalRGroups.AlertEventsGroup,
            cancellationToken: TestContext.Current.CancellationToken);
        await nonRecipientConnection.InvokeAsync(
            "JoinGroup",
            Constants.WellKnownSignalRGroups.AlertEventsGroup,
            cancellationToken: TestContext.Current.CancellationToken);

        var alertEvent = new AlertEvent(
            Guid.CreateVersion7(),
            AlertType.PlatformUnreachable,
            AlertSeverity.Critical,
            new PlatformUnreachableAlertInfo("signalr-platform", platformId, "tcp://signalr"),
            platformId,
            "signalr-platform",
            AlertResourceType.Platform);
        var dispatcher = Services.GetRequiredService<IApplicationHubDispatcher>();
        await dispatcher.SendTriggeredAlertEvent(alertEvent, [recipient.UserId]);
        var hubContext = Services.GetRequiredService<IHubContext<ApplicationHub>>();
        await hubContext.Clients.Client(nonRecipientConnection.ConnectionId!).SendAsync(
            "DeliveryBarrier",
            TestContext.Current.CancellationToken);

        await recipientEvent.Task.WaitAsync(
            TimeSpan.FromSeconds(5),
            TestContext.Current.CancellationToken);
        await nonRecipientBarrier.Task.WaitAsync(
            TimeSpan.FromSeconds(5),
            TestContext.Current.CancellationToken);
        Assert.False(nonRecipientEvent.Task.IsCompleted);
    }

    protected async Task RunPrivateGroupNameScenarioAsync()
    {
        var resourceId = Guid.CreateVersion7();
        var backupPolicyId = Guid.CreateVersion7();
        var platformId = Guid.CreateVersion7();
        var userId = Guid.CreateVersion7();
        var licenseId = Guid.CreateVersion7();
        var subject = await CreateAuthorizationSubjectAsync(
            resourceGrants:
            [
                new ResourceGrant(ResourceType.Deployment, resourceId, PermissionLevel.Read),
                new ResourceGrant(ResourceType.BackupPolicy, backupPolicyId, PermissionLevel.Read),
                new ResourceGrant(ResourceType.Platform, platformId, PermissionLevel.Read),
                new ResourceGrant(ResourceType.User, userId, PermissionLevel.Read),
                new ResourceGrant(ResourceType.License, licenseId, PermissionLevel.Read)
            ]);

        await AssertAuthorizationAllowedAsync(
            subject,
            Constants.WellKnownSignalRGroups.ActivityGroup(nameof(ActivityResourceType.Deployment), resourceId));
        await AssertAuthorizationAllowedAsync(
            subject,
            Constants.WellKnownSignalRGroups.ActivityGroup(nameof(ActivityResourceType.BackupPolicy), backupPolicyId));
        await AssertAuthorizationAllowedAsync(
            subject,
            Constants.WellKnownSignalRGroups.ActivityGroup(nameof(ActivityResourceType.Volume), platformId));
        await AssertAuthorizationDeniedAsync(
            subject,
            Constants.WellKnownSignalRGroups.ActivityGroup(nameof(ActivityResourceType.User), userId));
        await AssertAuthorizationDeniedAsync(
            subject,
            Constants.WellKnownSignalRGroups.ActivityGroup(nameof(ActivityResourceType.License), licenseId));
        await AssertAuthorizationAllowedAsync(subject, Constants.WellKnownSignalRGroups.AlertEventsGroup);
        await AssertAuthorizationDeniedAsync(subject, $"activity:{resourceId}");
        await AssertAuthorizationDeniedAsync(
            subject,
            Constants.WellKnownSignalRGroups.AlertEventsUserGroup(Guid.CreateVersion7()));
    }

    private async Task<AuthorizationSubject> CreateTeamResourceGrantSubjectAsync(
        ResourceType resourceType,
        Guid resourceId,
        PermissionLevel permissionLevel,
        SpecificPermission specificPermission = SpecificPermission.None)
    {
        var subject = await CreateAuthorizationSubjectAsync(createTeam: true);
        var access = ResourceAccess.Create(
            resourceType,
            resourceId,
            subject.TeamActorId!.Value,
            permissionLevel,
            specificPermission == SpecificPermission.None ? null : [specificPermission]);

        await using var scope = Services.CreateAsyncScope();
        var unitOfWork = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        await unitOfWork.ResourceAccesses.AddAsync(access, TestContext.Current.CancellationToken);
        await unitOfWork.CommitAsync(TestContext.Current.CancellationToken);
        return subject;
    }

    private async Task AssertJoinAllowedAsync(AuthorizationSubject subject, string groupId)
    {
        var connection = await CreateConnectionAsync(subject);
        await connection.InvokeAsync(
            "JoinGroup",
            groupId,
            cancellationToken: TestContext.Current.CancellationToken);
    }

    private async Task AssertJoinDeniedAsync(AuthorizationSubject subject, string groupId)
    {
        var connection = await CreateConnectionAsync(subject);
        await Assert.ThrowsAsync<HubException>(() => connection.InvokeAsync(
            "JoinGroup",
            groupId,
            cancellationToken: TestContext.Current.CancellationToken));
    }

    private Task AssertAuthorizationAllowedAsync(
        AuthorizationSubject subject,
        string groupId) =>
        AssertAuthorizationAsync(subject, groupId, expected: true);

    private Task AssertAuthorizationDeniedAsync(
        AuthorizationSubject subject,
        string groupId) =>
        AssertAuthorizationAsync(subject, groupId, expected: false);

    private async Task AssertAuthorizationAsync(
        AuthorizationSubject subject,
        string groupId,
        bool expected)
    {
        var principal = new ClaimsPrincipal(
            new ClaimsIdentity(
                [
                    new Claim(JwtRegisteredClaimNames.Sub, subject.UserId.ToString()),
                    new Claim("actorId", subject.ActorId.ToString())
                ],
                authenticationType: "IntegrationTest"));

        await using var scope = Services.CreateAsyncScope();
        var authorizationService =
            scope.ServiceProvider.GetRequiredService<ISignalRGroupAuthorizationService>();
        var actual = await authorizationService.CanJoinAsync(
            principal,
            groupId,
            TestContext.Current.CancellationToken);

        Assert.Equal(expected, actual);
    }

    private async Task<HubConnection> CreateConnectionAsync(
        AuthorizationSubject subject,
        IEnumerable<Claim>? claims = null)
    {
        var token = CreateJwtToken(subject.UserId, subject.ActorId, claims);
        var hubUrl = new Uri(Client.BaseAddress!, "/hubs/global");
        var connection = new HubConnectionBuilder()
            .WithUrl(hubUrl, options =>
            {
                options.AccessTokenProvider = () => Task.FromResult<string?>(token);
                options.HttpMessageHandlerFactory = _ => CreateServerHandler();
            })
            .Build();

        connections.Add(connection);
        await connection.StartAsync(TestContext.Current.CancellationToken);
        return connection;
    }

    public override async ValueTask DisposeAsync()
    {
        try
        {
            await Task.WhenAll(connections.Select(connection => connection.DisposeAsync().AsTask()));
        }
        finally
        {
            await base.DisposeAsync();
        }
    }
}

public sealed class ApplicationHubAuthorizationTestsPermissionMatrix(PostgresTestFixture fixture)
    : ApplicationHubAuthorizationTestBase(fixture)
{
    [Fact]
    public Task JoinGroup_EnforcesRepresentativePermissionMatrix()
        => RunRepresentativePermissionMatrixScenarioAsync();
}

public sealed class ApplicationHubAuthorizationTestsSpecificPermission(PostgresTestFixture fixture)
    : ApplicationHubAuthorizationTestBase(fixture)
{
    [Fact]
    public Task JoinGroup_RequiresSpecificLogAndTerminalPermissions()
        => RunSpecificPermissionScenarioAsync();
}

public sealed class ApplicationHubAuthorizationTestsSwarmService(PostgresTestFixture fixture)
    : ApplicationHubAuthorizationTestBase(fixture)
{
    [Fact]
    public Task JoinGroup_RequiresManagedServiceAndParentPlatformVisibility()
        => RunSwarmServiceParentVisibilityScenarioAsync();
}

public sealed class ApplicationHubAuthorizationTestsProtectedGroup(PostgresTestFixture fixture)
    : ApplicationHubAuthorizationTestBase(fixture)
{
    [Fact]
    public Task ProtectedGroup_DeliversOnlyToAuthorizedRecipient()
        => RunProtectedGroupDeliveryScenarioAsync();
}

public sealed class ApplicationHubAuthorizationTestsAlertEvent(PostgresTestFixture fixture)
    : ApplicationHubAuthorizationTestBase(fixture)
{
    [Fact]
    public Task AlertEvent_DeliversOnlyToSelectedUserGroup()
        => RunAlertEventDeliveryScenarioAsync();
}

public sealed class ApplicationHubAuthorizationTestsPrivateGroup(PostgresTestFixture fixture)
    : ApplicationHubAuthorizationTestBase(fixture)
{
    [Fact]
    public Task JoinGroup_RejectsUntypedActivityAndPrivateAlertGroupNames()
        => RunPrivateGroupNameScenarioAsync();
}
