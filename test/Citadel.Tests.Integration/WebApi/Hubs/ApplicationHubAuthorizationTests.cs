using Application.Services.Abstractions;
using Application.Services.SignalR;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities;
using Domain.Entities.Alerts;
using Domain.Entities.Identity;
using Domain.Entities.Platforms;
using Hosting.Common;
using Microsoft.AspNetCore.Http;
using Microsoft.AspNetCore.SignalR;
using Microsoft.AspNetCore.SignalR.Client;
using Microsoft.Extensions.DependencyInjection;
using System.Security.Claims;
using Tests.Integration.Helpers;
using WebApi.Hubs;
using WebApi.Routes.Endpoints.Resources.Alerters;

namespace Tests.Integration.WebApi.Hubs;

public sealed class ApplicationHubAuthorizationTests(PostgresTestFixture fixture) : IntegrationTestBase(fixture)
{
    private static readonly Guid AdminRoleId = Guid.Parse("30000000-0000-0000-0000-000000000001");
    private readonly List<HubConnection> connections = [];
    private Guid platformId;
    private string containerId = string.Empty;

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

        await unitOfWork.Platforms.AddAsync(platform, TestContext.Current.CancellationToken);
        await unitOfWork.Containers.AddAsync(container, TestContext.Current.CancellationToken);
        await unitOfWork.CommitAsync(TestContext.Current.CancellationToken);

        platformId = platform.Id;
        containerId = container.DockerContainerId[..12];
    }

    [Fact]
    public async Task JoinGroup_EnforcesRepresentativePermissionMatrix()
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

        await AssertJoinAllowedAsync(admin, Constants.WellKnownSignalRGroups.DeploymentGroup(resourceId));
        await AssertJoinAllowedAsync(directGlobal, Constants.WellKnownSignalRGroups.DeploymentGroup(resourceId));
        await AssertJoinAllowedAsync(teamGlobal, Constants.WellKnownSignalRGroups.DeploymentGroup(resourceId));
        await AssertJoinAllowedAsync(directGrant, Constants.WellKnownSignalRGroups.DeploymentGroup(resourceId));
        await AssertJoinAllowedAsync(teamGrant, Constants.WellKnownSignalRGroups.DeploymentGroup(resourceId));
        await AssertJoinDeniedAsync(directGrant, Constants.WellKnownSignalRGroups.DeploymentsGroup);
        await AssertJoinDeniedAsync(noGrant, Constants.WellKnownSignalRGroups.DeploymentGroup(resourceId));
        await AssertJoinDeniedAsync(disabled, Constants.WellKnownSignalRGroups.DeploymentGroup(resourceId));
    }

    [Fact]
    public async Task JoinGroup_RequiresSpecificLogAndTerminalPermissions()
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

        await AssertJoinDeniedAsync(readOnly, Constants.WellKnownSignalRGroups.StackLogGroup(stackId));
        await AssertJoinDeniedAsync(readOnly, Constants.WellKnownSignalRGroups.ContainerLogGroup(containerId));
        await AssertJoinDeniedAsync(
            readOnly,
            Constants.WellKnownSignalRGroups.ContainerExecGroup(containerId, "read-only-session"));
        await AssertJoinAllowedAsync(logReader, Constants.WellKnownSignalRGroups.StackLogGroup(stackId));
        await AssertJoinAllowedAsync(
            containerLogReader,
            Constants.WellKnownSignalRGroups.ContainerLogGroup(containerId));
        await AssertJoinAllowedAsync(
            terminalUser,
            Constants.WellKnownSignalRGroups.ContainerExecGroup(containerId, "terminal-session"));
    }

    [Fact]
    public async Task ProtectedGroup_DeliversOnlyToAuthorizedRecipient()
    {
        var resourceId = Guid.CreateVersion7();
        var authorized = await CreateAuthorizationSubjectAsync(
            resourceGrants: [new ResourceGrant(ResourceType.Deployment, resourceId, PermissionLevel.Read)]);
        var unauthorized = await CreateAuthorizationSubjectAsync();
        var authorizedEvent = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
        var unauthorizedEvent = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
        var authorizedConnection = await CreateConnectionAsync(authorized);
        var unauthorizedConnection = await CreateConnectionAsync(unauthorized);
        authorizedConnection.On("ProtectedEvent", () => authorizedEvent.TrySetResult());
        unauthorizedConnection.On("ProtectedEvent", () => unauthorizedEvent.TrySetResult());
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

        await authorizedEvent.Task.WaitAsync(
            TimeSpan.FromSeconds(5),
            TestContext.Current.CancellationToken);
        await Task.Delay(250, TestContext.Current.CancellationToken);
        Assert.False(unauthorizedEvent.Task.IsCompleted);
    }

    [Fact]
    public async Task AlertEvent_DeliversOnlyToSelectedUserGroup()
    {
        var recipient = await CreateAuthorizationSubjectAsync();
        var nonRecipient = await CreateAuthorizationSubjectAsync();
        var recipientEvent = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
        var nonRecipientEvent = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
        var recipientConnection = await CreateConnectionAsync(recipient);
        var nonRecipientConnection = await CreateConnectionAsync(nonRecipient);
        recipientConnection.On<AlertEventView>(
            "AlertEventReceived",
            _ => recipientEvent.TrySetResult());
        nonRecipientConnection.On<AlertEventView>(
            "AlertEventReceived",
            _ => nonRecipientEvent.TrySetResult());

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

        await recipientEvent.Task.WaitAsync(
            TimeSpan.FromSeconds(5),
            TestContext.Current.CancellationToken);
        await Task.Delay(250, TestContext.Current.CancellationToken);
        Assert.False(nonRecipientEvent.Task.IsCompleted);
    }

    [Fact]
    public async Task JoinGroup_RejectsUntypedActivityAndPrivateAlertGroupNames()
    {
        var resourceId = Guid.CreateVersion7();
        var subject = await CreateAuthorizationSubjectAsync(
            resourceGrants: [new ResourceGrant(ResourceType.Deployment, resourceId, PermissionLevel.Read)]);

        await AssertJoinAllowedAsync(
            subject,
            Constants.WellKnownSignalRGroups.ActivityGroup(nameof(ActivityResourceType.Deployment), resourceId));
        await AssertJoinAllowedAsync(subject, Constants.WellKnownSignalRGroups.AlertEventsGroup);
        await AssertJoinDeniedAsync(subject, $"activity:{resourceId}");
        await AssertJoinDeniedAsync(
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

    public new async ValueTask DisposeAsync()
    {
        foreach (var connection in connections)
            await connection.DisposeAsync();

        await base.DisposeAsync();
    }
}
