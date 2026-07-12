using System.Net;
using System.Net.Http.Headers;
using System.Text.Json;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Deployments;
using Domain.Entities.Git;
using Domain.Entities.Identity;
using Domain.Entities.Platforms;
using Domain.Entities.Registries;
using Domain.Entities.ResourceBindings;
using Domain.Entities.Stacks;
using Hosting.Common;
using Tests.Integration.Helpers;

namespace Tests.Integration.Application.Features.Lookup;

public class LookupTests(PostgresTestFixture fixture) : IntegrationTestBase(fixture)
{
    private Guid _platformId;
    private Guid _otherPlatformId;
    private Guid _visibleDeploymentId;
    private Guid _hiddenDeploymentId;
    private Guid _otherPlatformDeploymentId;
    private Guid _visibleRegistryId;
    private Guid _hiddenRegistryId;
    private Guid _extraRegistryId;
    private Guid _visibleGitRepositoryId;
    private Guid _hiddenGitRepositoryId;
    private Guid _extraGitRepositoryId;
    private Guid _gitStackId;
    private Guid _otherPlatformStackId;
    private Guid _visibleImageId;
    private Guid _hiddenImageId;
    private Guid _userLookupSourceId;
    private Guid _userLookupSourceActorId;
    private Guid _visibleTeamId;
    private Guid _hiddenTeamId;
    private Guid _extraTeamId;
    private Guid _visibleRoleId;
    private Guid _hiddenRoleId;
    private Guid _extraRoleId;
    private Guid _globalResourceBindingId;
    private Guid _deploymentResourceBindingId;
    private Guid _stackResourceBindingId;

    protected override async ValueTask SeedDbAsync(IUnitOfWork uow)
    {
        var platform = Fakes.GetDummyPlatform();
        var otherPlatform = new Platform(
            name: "platform-other",
            address: "https://platform-other.local",
            networkCount: 1,
            volumeCount: 2,
            imageCount: 3,
            cpuCount: 4,
            memTotal: 500,
            serverVersion: "1.0.0",
            agentVersion: "1.0.0",
            status: PlatformStatus.Online,
            connectorType: PlatformConnectorType.Agent,
            platformDescriptor: new DockerPlatformDescriptor(
                DaemonId: "other-daemon",
                ContainerCount: 5,
                ContainersRunning: 2,
                ContainersPaused: 2,
                ContainersStopped: 1));
        await uow.Platforms.AddAsync(platform, TestContext.Current.CancellationToken);
        await uow.Platforms.AddAsync(otherPlatform, TestContext.Current.CancellationToken);
        _platformId = platform.Id;
        _otherPlatformId = otherPlatform.Id;

        var visibleRegistry = new Registry(
            name: "registry-visible",
            registryHost: "registry-visible.local",
            status: RegistryStatus.Active,
            createdByActorId: Constants.SystemId,
            configuration: DockerHubRegistry.Create("visible", "token-visible"));
        var hiddenRegistry = new Registry(
            name: "registry-hidden",
            registryHost: "registry-hidden.local",
            status: RegistryStatus.Active,
            createdByActorId: Constants.SystemId,
            configuration: DockerHubRegistry.Create("hidden", "token-hidden"));
        var extraRegistry = new Registry(
            name: "registry-extra",
            registryHost: "registry-extra.local",
            status: RegistryStatus.Active,
            createdByActorId: Constants.SystemId,
            configuration: DockerHubRegistry.Create("extra", "token-extra"));
        await uow.Registries.AddAsync(visibleRegistry, TestContext.Current.CancellationToken);
        await uow.Registries.AddAsync(hiddenRegistry, TestContext.Current.CancellationToken);
        await uow.Registries.AddAsync(extraRegistry, TestContext.Current.CancellationToken);
        _visibleRegistryId = visibleRegistry.Id;
        _hiddenRegistryId = hiddenRegistry.Id;
        _extraRegistryId = extraRegistry.Id;

        var visibleDeployment = new Deployment(
            name: "deployment-visible",
            createdByActorId: Constants.SystemId,
            platformId: _platformId,
            spec: new DeploymentSpec(new ExternalImage(_visibleRegistryId, "nginx:visible"), UpdateBehavior.Notify));
        var hiddenDeployment = new Deployment(
            name: "deployment-hidden",
            createdByActorId: Constants.SystemId,
            platformId: _platformId,
            spec: new DeploymentSpec(new ExternalImage(_hiddenRegistryId, "nginx:hidden"), UpdateBehavior.Notify));
        var otherPlatformDeployment = new Deployment(
            name: "deployment-other-platform",
            createdByActorId: Constants.SystemId,
            platformId: _otherPlatformId,
            spec: new DeploymentSpec(new ExternalImage(_extraRegistryId, "nginx:other"), UpdateBehavior.Notify));
        await uow.Deployments.AddAsync(visibleDeployment, TestContext.Current.CancellationToken);
        await uow.Deployments.AddAsync(hiddenDeployment, TestContext.Current.CancellationToken);
        await uow.Deployments.AddAsync(otherPlatformDeployment, TestContext.Current.CancellationToken);
        _visibleDeploymentId = visibleDeployment.Id;
        _hiddenDeploymentId = hiddenDeployment.Id;
        _otherPlatformDeploymentId = otherPlatformDeployment.Id;

        var visibleGitRepository = new GitRepository(
            name: "git-visible",
            description: null,
            url: "https://github.com/citadel-p/git-visible.git",
            defaultBranch: "main",
            gitAccountId: null,
            createdByActorId: Constants.SystemId);
        var hiddenGitRepository = new GitRepository(
            name: "git-hidden",
            description: null,
            url: "https://github.com/citadel-p/git-hidden.git",
            defaultBranch: "main",
            gitAccountId: null,
            createdByActorId: Constants.SystemId);
        var extraGitRepository = new GitRepository(
            name: "git-extra",
            description: null,
            url: "https://github.com/citadel-p/git-extra.git",
            defaultBranch: "main",
            gitAccountId: null,
            createdByActorId: Constants.SystemId);
        await uow.GitRepositories.AddAsync(visibleGitRepository, TestContext.Current.CancellationToken);
        await uow.GitRepositories.AddAsync(hiddenGitRepository, TestContext.Current.CancellationToken);
        await uow.GitRepositories.AddAsync(extraGitRepository, TestContext.Current.CancellationToken);
        _visibleGitRepositoryId = visibleGitRepository.Id;
        _hiddenGitRepositoryId = hiddenGitRepository.Id;
        _extraGitRepositoryId = extraGitRepository.Id;

        var gitStack = Stack.Create(
            name: "stack-git-visible",
            createdByActorId: Constants.SystemId,
            StackSource: StackSource.Git,
            platformId: _platformId,
            spec: new GitStack(
                GitRepoId: _visibleGitRepositoryId,
                CommitSha: "abc123",
                Branch: "main",
                UpdateBehavior: StackUpdateBehavior.Notify,
                RegistryId: _visibleRegistryId));
        var otherPlatformStack = Stack.Create(
            name: "stack-other-platform",
            createdByActorId: Constants.SystemId,
            StackSource: StackSource.Git,
            platformId: _otherPlatformId,
            spec: new GitStack(
                GitRepoId: _extraGitRepositoryId,
                CommitSha: "def456",
                Branch: "main",
                UpdateBehavior: StackUpdateBehavior.Notify,
                RegistryId: _extraRegistryId));
        await uow.Stacks.AddAsync(gitStack, TestContext.Current.CancellationToken);
        await uow.Stacks.AddAsync(otherPlatformStack, TestContext.Current.CancellationToken);
        _gitStackId = gitStack.Id;
        _otherPlatformStackId = otherPlatformStack.Id;

        var visibleImage = new Domain.Entities.Image(
            name: "image-visible",
            tags: ["image-visible:latest"],
            dockerImageId: "docker-image-visible",
            size: 1234,
            containers: 0,
            platformId: _platformId,
            createdAt: DateTime.UtcNow,
            registryId: _visibleRegistryId);
        var hiddenImage = new Domain.Entities.Image(
            name: "image-hidden",
            tags: ["image-hidden:latest"],
            dockerImageId: "docker-image-hidden",
            size: 1234,
            containers: 0,
            platformId: _platformId,
            createdAt: DateTime.UtcNow,
            registryId: _hiddenRegistryId);
        await uow.Images.AddOrUpdateAsync(visibleImage, TestContext.Current.CancellationToken);
        await uow.Images.AddOrUpdateAsync(hiddenImage, TestContext.Current.CancellationToken);
        _visibleImageId = visibleImage.Id;
        _hiddenImageId = hiddenImage.Id;

        var userActor = Actor.Create(ActorType.User, new ActorMetadata("lookup-source-user"));
        var lookupUser = new User("lookup-source-user", "lookup-source-user@citadel.local", "password123", userActor.Id, Constants.SystemId);
        await uow.Actors.AddAsync(userActor, TestContext.Current.CancellationToken);
        await uow.Users.AddAsync(lookupUser, TestContext.Current.CancellationToken);
        _userLookupSourceId = lookupUser.Id;
        _userLookupSourceActorId = lookupUser.ActorId;

        var visibleTeamActor = Actor.Create(ActorType.Team, new ActorMetadata("team-visible"));
        var hiddenTeamActor = Actor.Create(ActorType.Team, new ActorMetadata("team-hidden"));
        var visibleTeam = Team.Create("team-visible", visibleTeamActor.Id);
        var hiddenTeam = Team.Create("team-hidden", hiddenTeamActor.Id);
        var extraTeamActor = Actor.Create(ActorType.Team, new ActorMetadata("team-extra"));
        var extraTeam = Team.Create("team-extra", extraTeamActor.Id);
        await uow.Actors.AddAsync(visibleTeamActor, TestContext.Current.CancellationToken);
        await uow.Actors.AddAsync(hiddenTeamActor, TestContext.Current.CancellationToken);
        await uow.Actors.AddAsync(extraTeamActor, TestContext.Current.CancellationToken);
        await uow.Teams.AddAsync(visibleTeam, TestContext.Current.CancellationToken);
        await uow.Teams.AddAsync(hiddenTeam, TestContext.Current.CancellationToken);
        await uow.Teams.AddAsync(extraTeam, TestContext.Current.CancellationToken);
        await uow.Teams.AddMemberAsync(visibleTeam.Id, lookupUser.Id, TestContext.Current.CancellationToken);
        await uow.Teams.AddMemberAsync(hiddenTeam.Id, lookupUser.Id, TestContext.Current.CancellationToken);
        _visibleTeamId = visibleTeam.Id;
        _hiddenTeamId = hiddenTeam.Id;
        _extraTeamId = extraTeam.Id;

        var visibleRole = Role.Create("role-visible", RoleType.Custom);
        var hiddenRole = Role.Create("role-hidden", RoleType.Custom);
        var extraRole = Role.Create("role-extra", RoleType.Custom);
        await uow.Roles.AddAsync(visibleRole, TestContext.Current.CancellationToken);
        await uow.Roles.AddAsync(hiddenRole, TestContext.Current.CancellationToken);
        await uow.Roles.AddAsync(extraRole, TestContext.Current.CancellationToken);
        await uow.Roles.AddActorRoleAsync(lookupUser.ActorId, visibleRole.Id, TestContext.Current.CancellationToken);
        await uow.Roles.AddActorRoleAsync(lookupUser.ActorId, hiddenRole.Id, TestContext.Current.CancellationToken);
        _visibleRoleId = visibleRole.Id;
        _hiddenRoleId = hiddenRole.Id;
        _extraRoleId = extraRole.Id;

        var globalResourceBinding = new ResourceBinding(
            Name: "GLOBAL_LOOKUP_VALUE",
            Kind: ResourceBindingKind.Variable,
            Scope: ResourceBindingScope.Global,
            ResourceId: null,
            Value: "global",
            SecretId: null);
        var deploymentResourceBinding = new ResourceBinding(
            Name: "DEPLOYMENT_LOOKUP_VALUE",
            Kind: ResourceBindingKind.Variable,
            Scope: ResourceBindingScope.Deployment,
            ResourceId: _visibleDeploymentId,
            Value: "deployment",
            SecretId: null);
        var stackResourceBinding = new ResourceBinding(
            Name: "STACK_LOOKUP_VALUE",
            Kind: ResourceBindingKind.Variable,
            Scope: ResourceBindingScope.Stack,
            ResourceId: _gitStackId,
            Value: "stack",
            SecretId: null);
        await uow.ResourceBindings.AddAsync(globalResourceBinding, TestContext.Current.CancellationToken);
        await uow.ResourceBindings.AddAsync(deploymentResourceBinding, TestContext.Current.CancellationToken);
        await uow.ResourceBindings.AddAsync(stackResourceBinding, TestContext.Current.CancellationToken);
        _globalResourceBindingId = globalResourceBinding.Id;
        _deploymentResourceBindingId = deploymentResourceBinding.Id;
        _stackResourceBindingId = stackResourceBinding.Id;

        await uow.CommitAsync(TestContext.Current.CancellationToken);
    }

    [Fact]
    public async Task Lookup_Platform_To_Image_Should_Return_Only_Visible_Images()
    {
        var subject = await CreateAuthorizationSubjectAsync(
            resourceGrants:
            [
                new ResourceGrant(ResourceType.Platform, _platformId, PermissionLevel.Read)
            ]);

        Client.DefaultRequestHeaders.Authorization = new AuthenticationHeaderValue(
            "Bearer",
            CreateJwtToken(subject.UserId, subject.ActorId));

        var response = await Client.GetAsync($"/api/v1/lookup?sourceResourceType=Platform&sourceResourceId={_platformId}&targetResourceType=Image", TestContext.Current.CancellationToken);
        var responseBody = await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);
        Assert.True(response.IsSuccessStatusCode, responseBody);

        using var document = await JsonDocument.ParseAsync(
            new MemoryStream(System.Text.Encoding.UTF8.GetBytes(responseBody)),
            cancellationToken: TestContext.Current.CancellationToken);

        var items = document.RootElement;
        Assert.Equal(2, items.GetArrayLength());
        Assert.Contains(items.EnumerateArray(), item => item.GetProperty("id").GetGuid() == _visibleImageId);
        Assert.Contains(items.EnumerateArray(), item => item.GetProperty("id").GetGuid() == _hiddenImageId);
    }

    [Fact]
    public async Task Lookup_Platform_To_Deployment_Should_Preserve_Linked_Deployments_When_Source_Is_Accessible()
    {
        var subject = await CreateAuthorizationSubjectAsync(
            resourceGrants:
            [
                new ResourceGrant(ResourceType.Platform, _platformId, PermissionLevel.Read),
                new ResourceGrant(ResourceType.Deployment, _visibleDeploymentId, PermissionLevel.Read)
            ]);

        Client.DefaultRequestHeaders.Authorization = new AuthenticationHeaderValue(
            "Bearer",
            CreateJwtToken(subject.UserId, subject.ActorId));

        var response = await Client.GetAsync($"/api/v1/lookup?sourceResourceType=Platform&sourceResourceId={_platformId}&targetResourceType=Deployment", TestContext.Current.CancellationToken);
        response.EnsureSuccessStatusCode();

        using var document = await JsonDocument.ParseAsync(
            await response.Content.ReadAsStreamAsync(TestContext.Current.CancellationToken),
            cancellationToken: TestContext.Current.CancellationToken);

        var items = document.RootElement;
        Assert.Equal(2, items.GetArrayLength());
        Assert.Contains(items.EnumerateArray(), item => item.GetProperty("id").GetGuid() == _visibleDeploymentId);
        Assert.Contains(items.EnumerateArray(), item => item.GetProperty("id").GetGuid() == _hiddenDeploymentId);
    }

    [Fact]
    public async Task Lookup_Platform_To_Deployment_Should_Return_Authorized_Deployments_And_Linked_Deployments_Without_Duplicates()
    {
        var subject = await CreateAuthorizationSubjectAsync(
            resourceGrants:
            [
                new ResourceGrant(ResourceType.Platform, _platformId, PermissionLevel.Read),
                new ResourceGrant(ResourceType.Deployment, _otherPlatformDeploymentId, PermissionLevel.Read)
            ]);

        Client.DefaultRequestHeaders.Authorization = new AuthenticationHeaderValue(
            "Bearer",
            CreateJwtToken(subject.UserId, subject.ActorId));

        var response = await Client.GetAsync($"/api/v1/lookup?sourceResourceType=Platform&sourceResourceId={_platformId}&targetResourceType=Deployment", TestContext.Current.CancellationToken);
        response.EnsureSuccessStatusCode();

        using var document = await JsonDocument.ParseAsync(
            await response.Content.ReadAsStreamAsync(TestContext.Current.CancellationToken),
            cancellationToken: TestContext.Current.CancellationToken);

        var items = document.RootElement.EnumerateArray().ToArray();
        Assert.Equal(3, items.Length);
        Assert.Contains(items, item => item.GetProperty("id").GetGuid() == _visibleDeploymentId);
        Assert.Contains(items, item => item.GetProperty("id").GetGuid() == _hiddenDeploymentId);
        Assert.Contains(items, item => item.GetProperty("id").GetGuid() == _otherPlatformDeploymentId);
        Assert.Equal(3, items.Select(item => item.GetProperty("id").GetGuid()).Distinct().Count());
    }

    [Fact]
    public async Task Lookup_Deployment_To_Registry_Should_Return_Only_Visible_Registries()
    {
        var subject = await CreateAuthorizationSubjectAsync(
            resourceGrants:
            [
                new ResourceGrant(ResourceType.Deployment, _visibleDeploymentId, PermissionLevel.Read),
                new ResourceGrant(ResourceType.Registry, _visibleRegistryId, PermissionLevel.Read)
            ]);

        Client.DefaultRequestHeaders.Authorization = new AuthenticationHeaderValue(
            "Bearer",
            CreateJwtToken(subject.UserId, subject.ActorId));

        var response = await Client.GetAsync($"/api/v1/lookup?sourceResourceType=Deployment&sourceResourceId={_visibleDeploymentId}&targetResourceType=Registry", TestContext.Current.CancellationToken);
        var responseBody = await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);
        Assert.True(response.IsSuccessStatusCode, responseBody);

        using var document = await JsonDocument.ParseAsync(
            new MemoryStream(System.Text.Encoding.UTF8.GetBytes(responseBody)),
            cancellationToken: TestContext.Current.CancellationToken);

        var items = document.RootElement;
        Assert.Single(items.EnumerateArray());
        Assert.Equal(_visibleRegistryId, items[0].GetProperty("id").GetGuid());
    }

    [Fact]
    public async Task Lookup_Deployment_To_Registry_Should_Preserve_Referenced_Registry_Without_Direct_Target_Access()
    {
        var subject = await CreateAuthorizationSubjectAsync(
            resourceGrants:
            [
                new ResourceGrant(ResourceType.Deployment, _visibleDeploymentId, PermissionLevel.Read)
            ]);

        Client.DefaultRequestHeaders.Authorization = new AuthenticationHeaderValue(
            "Bearer",
            CreateJwtToken(subject.UserId, subject.ActorId));

        var response = await Client.GetAsync($"/api/v1/lookup?sourceResourceType=Deployment&sourceResourceId={_visibleDeploymentId}&targetResourceType=Registry", TestContext.Current.CancellationToken);
        var responseBody = await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);
        Assert.True(response.IsSuccessStatusCode, responseBody);

        using var document = await JsonDocument.ParseAsync(
            new MemoryStream(System.Text.Encoding.UTF8.GetBytes(responseBody)),
            cancellationToken: TestContext.Current.CancellationToken);

        var items = document.RootElement;
        Assert.Single(items.EnumerateArray());
        Assert.Equal(_visibleRegistryId, items[0].GetProperty("id").GetGuid());
    }

    [Fact]
    public async Task Lookup_Deployment_To_Registry_Should_Return_Authorized_Registries_And_The_Referenced_Registry_Without_Duplicates()
    {
        var subject = await CreateAuthorizationSubjectAsync(
            resourceGrants:
            [
                new ResourceGrant(ResourceType.Deployment, _visibleDeploymentId, PermissionLevel.Read),
                new ResourceGrant(ResourceType.Registry, _hiddenRegistryId, PermissionLevel.Read)
            ]);

        Client.DefaultRequestHeaders.Authorization = new AuthenticationHeaderValue(
            "Bearer",
            CreateJwtToken(subject.UserId, subject.ActorId));

        var response = await Client.GetAsync($"/api/v1/lookup?sourceResourceType=Deployment&sourceResourceId={_visibleDeploymentId}&targetResourceType=Registry", TestContext.Current.CancellationToken);
        var responseBody = await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);
        Assert.True(response.IsSuccessStatusCode, responseBody);

        using var document = await JsonDocument.ParseAsync(
            new MemoryStream(System.Text.Encoding.UTF8.GetBytes(responseBody)),
            cancellationToken: TestContext.Current.CancellationToken);

        var items = document.RootElement.EnumerateArray().ToArray();
        Assert.Equal(2, items.Length);
        Assert.Contains(items, item => item.GetProperty("id").GetGuid() == _visibleRegistryId);
        Assert.Contains(items, item => item.GetProperty("id").GetGuid() == _hiddenRegistryId);
        Assert.Equal(2, items.Select(item => item.GetProperty("id").GetGuid()).Distinct().Count());
    }

    [Fact]
    public async Task Lookup_Deployment_To_ResourceBinding_Should_Return_Effective_Bindings_With_Resource_Read_Access()
    {
        var subject = await CreateAuthorizationSubjectAsync(
            resourceGrants:
            [
                new ResourceGrant(ResourceType.Deployment, _visibleDeploymentId, PermissionLevel.Read)
            ]);

        Client.DefaultRequestHeaders.Authorization = new AuthenticationHeaderValue(
            "Bearer",
            CreateJwtToken(subject.UserId, subject.ActorId));

        var response = await Client.GetAsync($"/api/v1/lookup?sourceResourceType=Deployment&sourceResourceId={_visibleDeploymentId}&targetResourceType=ResourceBinding", TestContext.Current.CancellationToken);
        var responseBody = await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);
        Assert.True(response.IsSuccessStatusCode, responseBody);

        using var document = await JsonDocument.ParseAsync(
            new MemoryStream(System.Text.Encoding.UTF8.GetBytes(responseBody)),
            cancellationToken: TestContext.Current.CancellationToken);

        var items = document.RootElement.EnumerateArray().ToArray();
        Assert.Equal(2, items.Length);
        Assert.Contains(items, item => item.GetProperty("id").GetGuid() == _globalResourceBindingId && item.GetProperty("name").GetString() == "GLOBAL_LOOKUP_VALUE");
        Assert.Contains(items, item => item.GetProperty("id").GetGuid() == _deploymentResourceBindingId && item.GetProperty("name").GetString() == "DEPLOYMENT_LOOKUP_VALUE");
    }

    [Fact]
    public async Task Lookup_Deployment_To_Platform_Should_Preserve_Referenced_Platform_Without_Direct_Target_Access()
    {
        var subject = await CreateAuthorizationSubjectAsync(
            resourceGrants:
            [
                new ResourceGrant(ResourceType.Deployment, _visibleDeploymentId, PermissionLevel.Read)
            ]);

        Client.DefaultRequestHeaders.Authorization = new AuthenticationHeaderValue(
            "Bearer",
            CreateJwtToken(subject.UserId, subject.ActorId));

        var response = await Client.GetAsync($"/api/v1/lookup?sourceResourceType=Deployment&sourceResourceId={_visibleDeploymentId}&targetResourceType=Platform", TestContext.Current.CancellationToken);
        var responseBody = await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);
        Assert.True(response.IsSuccessStatusCode, responseBody);

        using var document = await JsonDocument.ParseAsync(
            new MemoryStream(System.Text.Encoding.UTF8.GetBytes(responseBody)),
            cancellationToken: TestContext.Current.CancellationToken);

        var items = document.RootElement;
        Assert.Single(items.EnumerateArray());
        Assert.Equal(_platformId, items[0].GetProperty("id").GetGuid());
    }

    [Fact]
    public async Task Lookup_Deployment_To_Platform_Should_Return_Authorized_Platforms_And_Referenced_Platform_Without_Duplicates()
    {
        var subject = await CreateAuthorizationSubjectAsync(
            resourceGrants:
            [
                new ResourceGrant(ResourceType.Deployment, _visibleDeploymentId, PermissionLevel.Read),
                new ResourceGrant(ResourceType.Platform, _otherPlatformId, PermissionLevel.Read)
            ]);

        Client.DefaultRequestHeaders.Authorization = new AuthenticationHeaderValue(
            "Bearer",
            CreateJwtToken(subject.UserId, subject.ActorId));

        var response = await Client.GetAsync($"/api/v1/lookup?sourceResourceType=Deployment&sourceResourceId={_visibleDeploymentId}&targetResourceType=Platform", TestContext.Current.CancellationToken);
        var responseBody = await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);
        Assert.True(response.IsSuccessStatusCode, responseBody);

        using var document = await JsonDocument.ParseAsync(
            new MemoryStream(System.Text.Encoding.UTF8.GetBytes(responseBody)),
            cancellationToken: TestContext.Current.CancellationToken);

        var items = document.RootElement.EnumerateArray().ToArray();
        Assert.Equal(2, items.Length);
        Assert.Contains(items, item => item.GetProperty("id").GetGuid() == _platformId);
        Assert.Contains(items, item => item.GetProperty("id").GetGuid() == _otherPlatformId);
        Assert.Equal(2, items.Select(item => item.GetProperty("id").GetGuid()).Distinct().Count());
    }

    [Fact]
    public async Task Lookup_Deployment_To_Image_Should_Return_Platform_Images_Without_Direct_Target_Access()
    {
        var subject = await CreateAuthorizationSubjectAsync(
            resourceGrants:
            [
                new ResourceGrant(ResourceType.Deployment, _visibleDeploymentId, PermissionLevel.Read)
            ]);

        Client.DefaultRequestHeaders.Authorization = new AuthenticationHeaderValue(
            "Bearer",
            CreateJwtToken(subject.UserId, subject.ActorId));

        var response = await Client.GetAsync($"/api/v1/lookup?sourceResourceType=Deployment&sourceResourceId={_visibleDeploymentId}&targetResourceType=Image&platformId={_platformId}", TestContext.Current.CancellationToken);
        var responseBody = await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);
        Assert.True(response.IsSuccessStatusCode, responseBody);

        using var document = await JsonDocument.ParseAsync(
            new MemoryStream(System.Text.Encoding.UTF8.GetBytes(responseBody)),
            cancellationToken: TestContext.Current.CancellationToken);

        var items = document.RootElement;
        Assert.Equal(2, items.GetArrayLength());
        Assert.Contains(items.EnumerateArray(), item => item.GetProperty("id").GetGuid() == _visibleImageId);
        Assert.Contains(items.EnumerateArray(), item => item.GetProperty("id").GetGuid() == _hiddenImageId);
    }

    [Fact]
    public async Task Lookup_Image_To_Registry_Without_SourceId_Should_Return_Only_Visible_Registries()
    {
        var subject = await CreateAuthorizationSubjectAsync(
            resourceGrants: [new ResourceGrant(ResourceType.Registry, _visibleRegistryId, PermissionLevel.Read)]);

        Client.DefaultRequestHeaders.Authorization = new AuthenticationHeaderValue(
            "Bearer",
            CreateJwtToken(subject.UserId, subject.ActorId));

        var response = await Client.GetAsync("/api/v1/lookup?sourceResourceType=Image&targetResourceType=Registry", TestContext.Current.CancellationToken);
        var responseBody = await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);
        Assert.True(response.IsSuccessStatusCode, responseBody);

        using var document = await JsonDocument.ParseAsync(
            new MemoryStream(System.Text.Encoding.UTF8.GetBytes(responseBody)),
            cancellationToken: TestContext.Current.CancellationToken);

        var items = document.RootElement;
        Assert.Single(items.EnumerateArray());
        Assert.Equal(_visibleRegistryId, items[0].GetProperty("id").GetGuid());
    }

    [Fact]
    public async Task Lookup_Stack_To_GitRepository_Should_Return_Only_Visible_GitRepository()
    {
        var subject = await CreateAuthorizationSubjectAsync(
            resourceGrants:
            [
                new ResourceGrant(ResourceType.Stack, _gitStackId, PermissionLevel.Read),
                new ResourceGrant(ResourceType.GitRepository, _visibleGitRepositoryId, PermissionLevel.Read)
            ]);

        Client.DefaultRequestHeaders.Authorization = new AuthenticationHeaderValue(
            "Bearer",
            CreateJwtToken(subject.UserId, subject.ActorId));

        var response = await Client.GetAsync($"/api/v1/lookup?sourceResourceType=Stack&sourceResourceId={_gitStackId}&targetResourceType=GitRepository", TestContext.Current.CancellationToken);
        response.EnsureSuccessStatusCode();

        using var document = await JsonDocument.ParseAsync(
            await response.Content.ReadAsStreamAsync(TestContext.Current.CancellationToken),
            cancellationToken: TestContext.Current.CancellationToken);

        var items = document.RootElement;
        Assert.Equal(1, items.GetArrayLength());
        Assert.Equal(_visibleGitRepositoryId, items[0].GetProperty("id").GetGuid());
        Assert.Equal("git-visible", items[0].GetProperty("name").GetString());
    }

    [Fact]
    public async Task Lookup_Stack_To_GitRepository_Should_Preserve_Referenced_GitRepository_Without_Direct_Target_Access()
    {
        var subject = await CreateAuthorizationSubjectAsync(
            resourceGrants:
            [
                new ResourceGrant(ResourceType.Stack, _gitStackId, PermissionLevel.Read)
            ]);

        Client.DefaultRequestHeaders.Authorization = new AuthenticationHeaderValue(
            "Bearer",
            CreateJwtToken(subject.UserId, subject.ActorId));

        var response = await Client.GetAsync($"/api/v1/lookup?sourceResourceType=Stack&sourceResourceId={_gitStackId}&targetResourceType=GitRepository", TestContext.Current.CancellationToken);
        var responseBody = await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);
        Assert.True(response.IsSuccessStatusCode, responseBody);

        using var document = await JsonDocument.ParseAsync(
            new MemoryStream(System.Text.Encoding.UTF8.GetBytes(responseBody)),
            cancellationToken: TestContext.Current.CancellationToken);

        var items = document.RootElement;
        Assert.Single(items.EnumerateArray());
        Assert.Equal(_visibleGitRepositoryId, items[0].GetProperty("id").GetGuid());
    }

    [Fact]
    public async Task Lookup_Stack_To_GitRepository_Should_Return_Authorized_GitRepositories_And_Referenced_GitRepository_Without_Duplicates()
    {
        var subject = await CreateAuthorizationSubjectAsync(
            resourceGrants:
            [
                new ResourceGrant(ResourceType.Stack, _gitStackId, PermissionLevel.Read),
                new ResourceGrant(ResourceType.GitRepository, _hiddenGitRepositoryId, PermissionLevel.Read)
            ]);

        Client.DefaultRequestHeaders.Authorization = new AuthenticationHeaderValue(
            "Bearer",
            CreateJwtToken(subject.UserId, subject.ActorId));

        var response = await Client.GetAsync($"/api/v1/lookup?sourceResourceType=Stack&sourceResourceId={_gitStackId}&targetResourceType=GitRepository", TestContext.Current.CancellationToken);
        var responseBody = await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);
        Assert.True(response.IsSuccessStatusCode, responseBody);

        using var document = await JsonDocument.ParseAsync(
            new MemoryStream(System.Text.Encoding.UTF8.GetBytes(responseBody)),
            cancellationToken: TestContext.Current.CancellationToken);

        var items = document.RootElement.EnumerateArray().ToArray();
        Assert.Equal(2, items.Length);
        Assert.Contains(items, item => item.GetProperty("id").GetGuid() == _visibleGitRepositoryId);
        Assert.Contains(items, item => item.GetProperty("id").GetGuid() == _hiddenGitRepositoryId);
        Assert.Equal(2, items.Select(item => item.GetProperty("id").GetGuid()).Distinct().Count());
    }

    [Fact]
    public async Task Lookup_Alert_To_GitRepository_Should_Return_Only_Visible_GitRepositories()
    {
        var subject = await CreateAuthorizationSubjectAsync(
            resourceGrants:
            [
                new ResourceGrant(ResourceType.GitRepository, _visibleGitRepositoryId, PermissionLevel.Read)
            ]);

        Client.DefaultRequestHeaders.Authorization = new AuthenticationHeaderValue(
            "Bearer",
            CreateJwtToken(subject.UserId, subject.ActorId));

        var response = await Client.GetAsync("/api/v1/lookup?sourceResourceType=Alert&targetResourceType=GitRepository", TestContext.Current.CancellationToken);
        var responseBody = await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);
        Assert.True(response.IsSuccessStatusCode, responseBody);

        using var document = await JsonDocument.ParseAsync(
            new MemoryStream(System.Text.Encoding.UTF8.GetBytes(responseBody)),
            cancellationToken: TestContext.Current.CancellationToken);

        var items = document.RootElement;
        Assert.Equal(1, items.GetArrayLength());
        Assert.Equal(_visibleGitRepositoryId, items[0].GetProperty("id").GetGuid());
        Assert.Equal("git-visible", items[0].GetProperty("name").GetString());
    }

    [Fact]
    public async Task Lookup_Stack_To_Platform_Should_Return_Authorized_Platforms_And_Referenced_Platform_Without_Duplicates()
    {
        var subject = await CreateAuthorizationSubjectAsync(
            resourceGrants:
            [
                new ResourceGrant(ResourceType.Stack, _gitStackId, PermissionLevel.Read),
                new ResourceGrant(ResourceType.Platform, _otherPlatformId, PermissionLevel.Read)
            ]);

        Client.DefaultRequestHeaders.Authorization = new AuthenticationHeaderValue(
            "Bearer",
            CreateJwtToken(subject.UserId, subject.ActorId));

        var response = await Client.GetAsync($"/api/v1/lookup?sourceResourceType=Stack&sourceResourceId={_gitStackId}&targetResourceType=Platform", TestContext.Current.CancellationToken);
        var responseBody = await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);
        Assert.True(response.IsSuccessStatusCode, responseBody);

        using var document = await JsonDocument.ParseAsync(
            new MemoryStream(System.Text.Encoding.UTF8.GetBytes(responseBody)),
            cancellationToken: TestContext.Current.CancellationToken);

        var items = document.RootElement.EnumerateArray().ToArray();
        Assert.Equal(2, items.Length);
        Assert.Contains(items, item => item.GetProperty("id").GetGuid() == _platformId);
        Assert.Contains(items, item => item.GetProperty("id").GetGuid() == _otherPlatformId);
        Assert.Equal(2, items.Select(item => item.GetProperty("id").GetGuid()).Distinct().Count());
    }

    [Fact]
    public async Task Lookup_Stack_To_Registry_Should_Return_Authorized_Registries_And_Referenced_Registry_Without_Duplicates()
    {
        var subject = await CreateAuthorizationSubjectAsync(
            resourceGrants:
            [
                new ResourceGrant(ResourceType.Stack, _gitStackId, PermissionLevel.Read),
                new ResourceGrant(ResourceType.Registry, _hiddenRegistryId, PermissionLevel.Read)
            ]);

        Client.DefaultRequestHeaders.Authorization = new AuthenticationHeaderValue(
            "Bearer",
            CreateJwtToken(subject.UserId, subject.ActorId));

        var response = await Client.GetAsync($"/api/v1/lookup?sourceResourceType=Stack&sourceResourceId={_gitStackId}&targetResourceType=Registry", TestContext.Current.CancellationToken);
        var responseBody = await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);
        Assert.True(response.IsSuccessStatusCode, responseBody);

        using var document = await JsonDocument.ParseAsync(
            new MemoryStream(System.Text.Encoding.UTF8.GetBytes(responseBody)),
            cancellationToken: TestContext.Current.CancellationToken);

        var items = document.RootElement.EnumerateArray().ToArray();
        Assert.Equal(2, items.Length);
        Assert.Contains(items, item => item.GetProperty("id").GetGuid() == _visibleRegistryId);
        Assert.Contains(items, item => item.GetProperty("id").GetGuid() == _hiddenRegistryId);
        Assert.Equal(2, items.Select(item => item.GetProperty("id").GetGuid()).Distinct().Count());
    }

    [Fact]
    public async Task Lookup_Stack_To_ResourceBinding_Should_Return_Effective_Bindings_With_Resource_Read_Access()
    {
        var subject = await CreateAuthorizationSubjectAsync(
            resourceGrants:
            [
                new ResourceGrant(ResourceType.Stack, _gitStackId, PermissionLevel.Read)
            ]);

        Client.DefaultRequestHeaders.Authorization = new AuthenticationHeaderValue(
            "Bearer",
            CreateJwtToken(subject.UserId, subject.ActorId));

        var response = await Client.GetAsync($"/api/v1/lookup?sourceResourceType=Stack&sourceResourceId={_gitStackId}&targetResourceType=ResourceBinding", TestContext.Current.CancellationToken);
        var responseBody = await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);
        Assert.True(response.IsSuccessStatusCode, responseBody);

        using var document = await JsonDocument.ParseAsync(
            new MemoryStream(System.Text.Encoding.UTF8.GetBytes(responseBody)),
            cancellationToken: TestContext.Current.CancellationToken);

        var items = document.RootElement.EnumerateArray().ToArray();
        Assert.Equal(2, items.Length);
        Assert.Contains(items, item => item.GetProperty("id").GetGuid() == _globalResourceBindingId && item.GetProperty("name").GetString() == "GLOBAL_LOOKUP_VALUE");
        Assert.Contains(items, item => item.GetProperty("id").GetGuid() == _stackResourceBindingId && item.GetProperty("name").GetString() == "STACK_LOOKUP_VALUE");
    }

    [Fact]
    public async Task Lookup_User_To_Team_Should_Preserve_Linked_Teams_When_Source_Is_Accessible()
    {
        var subject = await CreateAuthorizationSubjectAsync(
            resourceGrants:
            [
                new ResourceGrant(ResourceType.User, _userLookupSourceId, PermissionLevel.Read),
                new ResourceGrant(ResourceType.Team, _visibleTeamId, PermissionLevel.Read)
            ]);

        Client.DefaultRequestHeaders.Authorization = new AuthenticationHeaderValue(
            "Bearer",
            CreateJwtToken(subject.UserId, subject.ActorId));

        var response = await Client.GetAsync($"/api/v1/lookup?sourceResourceType=User&sourceResourceId={_userLookupSourceId}&targetResourceType=Team", TestContext.Current.CancellationToken);
        var responseBody = await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);
        Assert.True(response.IsSuccessStatusCode, responseBody);

        using var document = await JsonDocument.ParseAsync(
            new MemoryStream(System.Text.Encoding.UTF8.GetBytes(responseBody)),
            cancellationToken: TestContext.Current.CancellationToken);

        var items = document.RootElement;
        Assert.Equal(2, items.GetArrayLength());
        Assert.Contains(items.EnumerateArray(), item => item.GetProperty("id").GetGuid() == _visibleTeamId);
        Assert.Contains(items.EnumerateArray(), item => item.GetProperty("id").GetGuid() == _hiddenTeamId);
    }

    [Fact]
    public async Task Lookup_User_To_Team_Should_Return_Authorized_Teams_And_Linked_Teams_Without_Duplicates()
    {
        var subject = await CreateAuthorizationSubjectAsync(
            resourceGrants:
            [
                new ResourceGrant(ResourceType.User, _userLookupSourceId, PermissionLevel.Read),
                new ResourceGrant(ResourceType.Team, _extraTeamId, PermissionLevel.Read)
            ]);

        Client.DefaultRequestHeaders.Authorization = new AuthenticationHeaderValue(
            "Bearer",
            CreateJwtToken(subject.UserId, subject.ActorId));

        var response = await Client.GetAsync($"/api/v1/lookup?sourceResourceType=User&sourceResourceId={_userLookupSourceId}&targetResourceType=Team", TestContext.Current.CancellationToken);
        var responseBody = await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);
        Assert.True(response.IsSuccessStatusCode, responseBody);

        using var document = await JsonDocument.ParseAsync(
            new MemoryStream(System.Text.Encoding.UTF8.GetBytes(responseBody)),
            cancellationToken: TestContext.Current.CancellationToken);

        var items = document.RootElement.EnumerateArray().ToArray();
        Assert.Equal(3, items.Length);
        Assert.Contains(items, item => item.GetProperty("id").GetGuid() == _visibleTeamId);
        Assert.Contains(items, item => item.GetProperty("id").GetGuid() == _hiddenTeamId);
        Assert.Contains(items, item => item.GetProperty("id").GetGuid() == _extraTeamId);
        Assert.Equal(3, items.Select(item => item.GetProperty("id").GetGuid()).Distinct().Count());
    }

    [Fact]
    public async Task Lookup_User_To_Team_Should_Preserve_Referenced_Team_Without_Direct_Target_Access()
    {
        var subject = await CreateAuthorizationSubjectAsync(
            resourceGrants:
            [
                new ResourceGrant(ResourceType.User, _userLookupSourceId, PermissionLevel.Read)
            ]);

        Client.DefaultRequestHeaders.Authorization = new AuthenticationHeaderValue(
            "Bearer",
            CreateJwtToken(subject.UserId, subject.ActorId));

        var response = await Client.GetAsync($"/api/v1/lookup?sourceResourceType=User&sourceResourceId={_userLookupSourceId}&targetResourceType=Team", TestContext.Current.CancellationToken);
        var responseBody = await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);
        Assert.True(response.IsSuccessStatusCode, responseBody);

        using var document = await JsonDocument.ParseAsync(
            new MemoryStream(System.Text.Encoding.UTF8.GetBytes(responseBody)),
            cancellationToken: TestContext.Current.CancellationToken);

        var items = document.RootElement;
        Assert.Equal(2, items.GetArrayLength());
        Assert.Contains(items.EnumerateArray(), item => item.GetProperty("id").GetGuid() == _visibleTeamId);
        Assert.Contains(items.EnumerateArray(), item => item.GetProperty("id").GetGuid() == _hiddenTeamId);
    }

    [Fact]
    public async Task Lookup_User_To_Role_Should_Preserve_Linked_Roles_When_Source_Is_Accessible()
    {
        var subject = await CreateAuthorizationSubjectAsync(
            resourceGrants:
            [
                new ResourceGrant(ResourceType.User, _userLookupSourceId, PermissionLevel.Read),
                new ResourceGrant(ResourceType.Role, _visibleRoleId, PermissionLevel.Read)
            ]);

        Client.DefaultRequestHeaders.Authorization = new AuthenticationHeaderValue(
            "Bearer",
            CreateJwtToken(subject.UserId, subject.ActorId));

        var response = await Client.GetAsync($"/api/v1/lookup?sourceResourceType=User&sourceResourceId={_userLookupSourceId}&targetResourceType=Role", TestContext.Current.CancellationToken);
        var responseBody = await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);
        Assert.True(response.IsSuccessStatusCode, responseBody);

        using var document = await JsonDocument.ParseAsync(
            new MemoryStream(System.Text.Encoding.UTF8.GetBytes(responseBody)),
            cancellationToken: TestContext.Current.CancellationToken);

        var items = document.RootElement;
        Assert.Equal(2, items.GetArrayLength());
        Assert.Contains(items.EnumerateArray(), item => item.GetProperty("id").GetGuid() == _visibleRoleId);
        Assert.Contains(items.EnumerateArray(), item => item.GetProperty("id").GetGuid() == _hiddenRoleId);
    }

    [Fact]
    public async Task Lookup_User_To_Role_Should_Return_Authorized_Roles_And_Linked_Roles_Without_Duplicates()
    {
        var subject = await CreateAuthorizationSubjectAsync(
            resourceGrants:
            [
                new ResourceGrant(ResourceType.User, _userLookupSourceId, PermissionLevel.Read),
                new ResourceGrant(ResourceType.Role, _extraRoleId, PermissionLevel.Read)
            ]);

        Client.DefaultRequestHeaders.Authorization = new AuthenticationHeaderValue(
            "Bearer",
            CreateJwtToken(subject.UserId, subject.ActorId));

        var response = await Client.GetAsync($"/api/v1/lookup?sourceResourceType=User&sourceResourceId={_userLookupSourceId}&targetResourceType=Role", TestContext.Current.CancellationToken);
        var responseBody = await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);
        Assert.True(response.IsSuccessStatusCode, responseBody);

        using var document = await JsonDocument.ParseAsync(
            new MemoryStream(System.Text.Encoding.UTF8.GetBytes(responseBody)),
            cancellationToken: TestContext.Current.CancellationToken);

        var items = document.RootElement.EnumerateArray().ToArray();
        Assert.Equal(3, items.Length);
        Assert.Contains(items, item => item.GetProperty("id").GetGuid() == _visibleRoleId);
        Assert.Contains(items, item => item.GetProperty("id").GetGuid() == _hiddenRoleId);
        Assert.Contains(items, item => item.GetProperty("id").GetGuid() == _extraRoleId);
        Assert.Equal(3, items.Select(item => item.GetProperty("id").GetGuid()).Distinct().Count());
    }

    [Fact]
    public async Task Lookup_Platform_To_Stack_Should_Return_Authorized_Stacks_And_Linked_Stacks_Without_Duplicates()
    {
        var subject = await CreateAuthorizationSubjectAsync(
            resourceGrants:
            [
                new ResourceGrant(ResourceType.Platform, _platformId, PermissionLevel.Read),
                new ResourceGrant(ResourceType.Stack, _otherPlatformStackId, PermissionLevel.Read)
            ]);

        Client.DefaultRequestHeaders.Authorization = new AuthenticationHeaderValue(
            "Bearer",
            CreateJwtToken(subject.UserId, subject.ActorId));

        var response = await Client.GetAsync($"/api/v1/lookup?sourceResourceType=Platform&sourceResourceId={_platformId}&targetResourceType=Stack", TestContext.Current.CancellationToken);
        var responseBody = await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);
        Assert.True(response.IsSuccessStatusCode, responseBody);

        using var document = await JsonDocument.ParseAsync(
            new MemoryStream(System.Text.Encoding.UTF8.GetBytes(responseBody)),
            cancellationToken: TestContext.Current.CancellationToken);

        var items = document.RootElement.EnumerateArray().ToArray();
        Assert.Equal(2, items.Length);
        Assert.Contains(items, item => item.GetProperty("id").GetGuid() == _gitStackId);
        Assert.Contains(items, item => item.GetProperty("id").GetGuid() == _otherPlatformStackId);
        Assert.Equal(2, items.Select(item => item.GetProperty("id").GetGuid()).Distinct().Count());
    }

    [Fact]
    public async Task Lookup_Platform_To_Registry_Should_Return_Authorized_Registries_And_Linked_Registries_Without_Duplicates()
    {
        var subject = await CreateAuthorizationSubjectAsync(
            resourceGrants:
            [
                new ResourceGrant(ResourceType.Platform, _platformId, PermissionLevel.Read),
                new ResourceGrant(ResourceType.Registry, _extraRegistryId, PermissionLevel.Read)
            ]);

        Client.DefaultRequestHeaders.Authorization = new AuthenticationHeaderValue(
            "Bearer",
            CreateJwtToken(subject.UserId, subject.ActorId));

        var response = await Client.GetAsync($"/api/v1/lookup?sourceResourceType=Platform&sourceResourceId={_platformId}&targetResourceType=Registry", TestContext.Current.CancellationToken);
        var responseBody = await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);
        Assert.True(response.IsSuccessStatusCode, responseBody);

        using var document = await JsonDocument.ParseAsync(
            new MemoryStream(System.Text.Encoding.UTF8.GetBytes(responseBody)),
            cancellationToken: TestContext.Current.CancellationToken);

        var items = document.RootElement.EnumerateArray().ToArray();
        Assert.Equal(3, items.Length);
        Assert.Contains(items, item => item.GetProperty("id").GetGuid() == _visibleRegistryId);
        Assert.Contains(items, item => item.GetProperty("id").GetGuid() == _hiddenRegistryId);
        Assert.Contains(items, item => item.GetProperty("id").GetGuid() == _extraRegistryId);
        Assert.Equal(3, items.Select(item => item.GetProperty("id").GetGuid()).Distinct().Count());
    }

    [Fact]
    public async Task Lookup_User_To_Role_Should_Preserve_Referenced_Roles_Without_Direct_Target_Access()
    {
        var subject = await CreateAuthorizationSubjectAsync(
            resourceGrants:
            [
                new ResourceGrant(ResourceType.User, _userLookupSourceId, PermissionLevel.Read)
            ]);

        Client.DefaultRequestHeaders.Authorization = new AuthenticationHeaderValue(
            "Bearer",
            CreateJwtToken(subject.UserId, subject.ActorId));

        var response = await Client.GetAsync($"/api/v1/lookup?sourceResourceType=User&sourceResourceId={_userLookupSourceId}&targetResourceType=Role", TestContext.Current.CancellationToken);
        var responseBody = await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);
        Assert.True(response.IsSuccessStatusCode, responseBody);

        using var document = await JsonDocument.ParseAsync(
            new MemoryStream(System.Text.Encoding.UTF8.GetBytes(responseBody)),
            cancellationToken: TestContext.Current.CancellationToken);

        var items = document.RootElement;
        Assert.Equal(2, items.GetArrayLength());
        Assert.Contains(items.EnumerateArray(), item => item.GetProperty("id").GetGuid() == _visibleRoleId);
        Assert.Contains(items.EnumerateArray(), item => item.GetProperty("id").GetGuid() == _hiddenRoleId);
    }

    [Fact]
    public async Task Lookup_User_Add_Mode_Should_Return_UserId_And_UserActor_Should_Return_ActorId()
    {
        var subject = await CreateAuthorizationSubjectAsync(
            resourceGrants:
            [
                new ResourceGrant(ResourceType.User, _userLookupSourceId, PermissionLevel.Read)
            ]);

        Client.DefaultRequestHeaders.Authorization = new AuthenticationHeaderValue(
            "Bearer",
            CreateJwtToken(subject.UserId, subject.ActorId));

        var userResponse = await Client.GetAsync("/api/v1/lookup?targetResourceType=User", TestContext.Current.CancellationToken);
        var userBody = await userResponse.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);
        Assert.True(userResponse.IsSuccessStatusCode, userBody);

        using (var document = await JsonDocument.ParseAsync(
            new MemoryStream(System.Text.Encoding.UTF8.GetBytes(userBody)),
            cancellationToken: TestContext.Current.CancellationToken))
        {
            var item = Assert.Single(document.RootElement.EnumerateArray());
            Assert.Equal(_userLookupSourceId, item.GetProperty("id").GetGuid());
        }

        var actorResponse = await Client.GetAsync("/api/v1/lookup?targetResourceType=UserActor", TestContext.Current.CancellationToken);
        var actorBody = await actorResponse.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);
        Assert.True(actorResponse.IsSuccessStatusCode, actorBody);

        using (var document = await JsonDocument.ParseAsync(
            new MemoryStream(System.Text.Encoding.UTF8.GetBytes(actorBody)),
            cancellationToken: TestContext.Current.CancellationToken))
        {
            var item = Assert.Single(document.RootElement.EnumerateArray());
            Assert.Equal(_userLookupSourceActorId, item.GetProperty("id").GetGuid());
        }
    }

    [Fact]
    public async Task Lookup_Should_Return_NotFound_When_Source_Is_Not_Accessible()
    {
        var subject = await CreateAuthorizationSubjectAsync();
        Client.DefaultRequestHeaders.Authorization = new AuthenticationHeaderValue(
            "Bearer",
            CreateJwtToken(subject.UserId, subject.ActorId));

        var response = await Client.GetAsync($"/api/v1/lookup?sourceResourceType=Platform&sourceResourceId={_platformId}&targetResourceType=Deployment", TestContext.Current.CancellationToken);

        Assert.Equal(HttpStatusCode.NotFound, response.StatusCode);
    }

    [Fact]
    public async Task Lookup_Should_Return_BadRequest_For_Unsupported_Pair()
    {
        var subject = await CreateAuthorizationSubjectAsync(
            resourceGrants: [new ResourceGrant(ResourceType.Platform, _platformId, PermissionLevel.Read)]);
        Client.DefaultRequestHeaders.Authorization = new AuthenticationHeaderValue(
            "Bearer",
            CreateJwtToken(subject.UserId, subject.ActorId));

        var response = await Client.GetAsync($"/api/v1/lookup?sourceResourceType=Platform&sourceResourceId={_platformId}&targetResourceType=User", TestContext.Current.CancellationToken);

        Assert.Equal(HttpStatusCode.BadRequest, response.StatusCode);
    }

    [Fact]
    public async Task Lookup_Team_Add_Mode_Should_Return_Only_Visible_Teams()
    {
        var subject = await CreateAuthorizationSubjectAsync(
            resourceGrants: [new ResourceGrant(ResourceType.Team, _visibleTeamId, PermissionLevel.Read)]);

        Client.DefaultRequestHeaders.Authorization = new AuthenticationHeaderValue(
            "Bearer",
            CreateJwtToken(subject.UserId, subject.ActorId));

        var response = await Client.GetAsync("/api/v1/lookup?targetResourceType=Team", TestContext.Current.CancellationToken);
        var responseBody = await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);
        Assert.True(response.IsSuccessStatusCode, responseBody);

        using var document = await JsonDocument.ParseAsync(
            new MemoryStream(System.Text.Encoding.UTF8.GetBytes(responseBody)),
            cancellationToken: TestContext.Current.CancellationToken);

        var items = document.RootElement;
        Assert.Equal(1, items.GetArrayLength());
        Assert.Equal(_visibleTeamId, items[0].GetProperty("id").GetGuid());
    }

    [Fact]
    public async Task Lookup_Image_Add_Mode_Should_Require_PlatformId()
    {
        var subject = await CreateAuthorizationSubjectAsync();
        Client.DefaultRequestHeaders.Authorization = new AuthenticationHeaderValue(
            "Bearer",
            CreateJwtToken(subject.UserId, subject.ActorId));

        var response = await Client.GetAsync("/api/v1/lookup?targetResourceType=Image", TestContext.Current.CancellationToken);

        Assert.Equal(HttpStatusCode.BadRequest, response.StatusCode);
    }

    [Fact]
    public async Task Lookup_Should_Return_BadRequest_When_SourceId_Is_Provided_Without_SourceType()
    {
        var subject = await CreateAuthorizationSubjectAsync();
        Client.DefaultRequestHeaders.Authorization = new AuthenticationHeaderValue(
            "Bearer",
            CreateJwtToken(subject.UserId, subject.ActorId));

        var response = await Client.GetAsync($"/api/v1/lookup?sourceResourceId={_platformId}&targetResourceType=Deployment", TestContext.Current.CancellationToken);

        Assert.Equal(HttpStatusCode.BadRequest, response.StatusCode);
    }
}
