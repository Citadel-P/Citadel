using System.Text;
using Application.Services;
using Application.TaskJobs;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Git;
using Hosting.Common;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.DependencyInjection.Extensions;
using Microsoft.Extensions.Hosting;
using Moq;
using System.Text.Json;
using System.Threading.Channels;
using Infrastructure.Persistence;

namespace Tests.Integration.Application.Features.GitRepositories;

public class GitRepositoryPatchTests(PostgresTestFixture fixture) : IntegrationTestBase(fixture)
{
    private readonly Mock<IRepoCacheManager> _repoCacheManagerMock = new();
    private readonly Channel<GitRepoSyncRequest> _gitSyncChannel = Channel.CreateUnbounded<GitRepoSyncRequest>();
    private Guid gitRepositoryId;
    private Guid gitAccountId;

    protected override void ConfigureTestServices(IServiceCollection services)
    {
        services.RemoveAll<IHostedService>();
        services.AddSingleton(_repoCacheManagerMock.Object);
        services.AddSingleton(_gitSyncChannel);
        services.AddSingleton(s => s.GetRequiredService<Channel<GitRepoSyncRequest>>().Reader);
        services.AddSingleton(s => s.GetRequiredService<Channel<GitRepoSyncRequest>>().Writer);
    }

    protected override async ValueTask SeedDbAsync(IUnitOfWork uow)
    {
        var gitAccount = new GitAccount(
            name: "GA-ORIGINAL",
            domain: "github.com",
            transport: GitTransport.Https,
            authType: GitAuthType.Basic,
            createdByActorId: Constants.SystemId,
            configuration: new BasicAuth("dummy-user", "dummy-password123"));

        await uow.GitAccounts.AddAsync(gitAccount, TestContext.Current.CancellationToken);

        var gitRepository = new GitRepository(
            name: "OriginalName",
            description: "Original description",
            url: "https://github.com/citadel-p/citadel.git",
            defaultBranch: "main",
            gitAccountId: gitAccount.Id,
            createdByActorId: Constants.SystemId);

        await uow.GitRepositories.AddAsync(gitRepository, TestContext.Current.CancellationToken);
        await uow.CommitAsync(TestContext.Current.CancellationToken);

        gitAccountId = gitAccount.Id;
        gitRepositoryId = gitRepository.Id;
    }

    [Fact]
    public async Task Patch_GitRepository_Should_Apply_MergePatch()
    {
        var patchJson = $$"""
        {
          "url": "https://github.com/citadel-p/citadel-api.git",
          "defaultBranch": "develop",
          "webhook": { "enabled": true },
          "gitAccountId": "{{gitAccountId}}"
        }
        """;
        var content = new StringContent(patchJson, Encoding.UTF8, "application/merge-patch+json");

        var response = await Client.PatchAsync($"/api/v1/gitRepositories/{gitRepositoryId}", content, cancellationToken: TestContext.Current.CancellationToken);
        response.EnsureSuccessStatusCode();

        var responseBody = await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var gitRepository = await uow.GitRepositories.GetAsync(gitRepositoryId, TestContext.Current.CancellationToken);

        Assert.NotNull(gitRepository);
        Assert.Equal("OriginalName", gitRepository.Name);
        Assert.Equal("Original description", gitRepository.Description);
        Assert.Equal("develop", gitRepository.DefaultBranch);
        Assert.Equal(ResourceControlState.Processing, gitRepository.ControlState);
        await VerifyJson(responseBody);
    }

    [Fact]
    public async Task Patch_GitRepository_Should_Persist_Sync_And_Webhook_Properties()
    {
        var patchJson = """
        {
          "syncMode": "PullInterval",
          "syncIntervalMinutes": 10,
          "webhook": {
            "enabled": true,
            "provider": "GitLab",
            "authScheme": "GitLabSignedToken",
            "secret": "secret-123"
          },
          "onPull": {
            "path": "./scripts",
            "commands": ["echo pull"]
          },
          "onClone": {
            "path": ".",
            "commands": ["echo clone"]
          }
        }
        """;
        var content = new StringContent(patchJson, Encoding.UTF8, "application/merge-patch+json");

        var response = await Client.PatchAsync($"/api/v1/gitRepositories/{gitRepositoryId}", content, cancellationToken: TestContext.Current.CancellationToken);
        response.EnsureSuccessStatusCode();

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var gitRepository = await uow.GitRepositories.GetAsync(gitRepositoryId, TestContext.Current.CancellationToken);

        Assert.NotNull(gitRepository);
        Assert.Equal(GitRepositorySyncMode.PullInterval, gitRepository.SyncMode);
        Assert.Equal(10, gitRepository.SyncIntervalMinutes);
        Assert.True(gitRepository.Webhook?.Enabled);
        Assert.Equal(WebhookProvider.GitLab, gitRepository.Webhook?.Provider);
        Assert.Equal(WebhookAuthScheme.GitLabSignedToken, gitRepository.Webhook?.AuthScheme);
        Assert.Equal("secret-123", gitRepository.Webhook?.Secret);
        Assert.Equal("./scripts", gitRepository.OnPull?.Path);
        Assert.Equal(["echo pull"], gitRepository.OnPull?.Commands);
        Assert.Equal(".", gitRepository.OnClone?.Path);
        Assert.Equal(["echo clone"], gitRepository.OnClone?.Commands);

        var persistedWebhookJson = await GetPersistedWebhookJsonAsync(gitRepositoryId);
        using (var persistedWebhookDocument = JsonDocument.Parse(persistedWebhookJson!))
        {
            var persistedWebhook = persistedWebhookDocument.RootElement;
            Assert.True(persistedWebhook.GetProperty("Enabled").GetBoolean());
            Assert.Equal("GitLab", persistedWebhook.GetProperty("Provider").GetString());
            Assert.Equal("GitLabSignedToken", persistedWebhook.GetProperty("AuthScheme").GetString());
            Assert.Equal("secret-123", persistedWebhook.GetProperty("Secret").GetString());
        }

        var configResponse = await Client.GetAsync($"/api/v1/gitRepositories/{gitRepositoryId}/_cfg", TestContext.Current.CancellationToken);
        configResponse.EnsureSuccessStatusCode();

        using var configDocument = await JsonDocument.ParseAsync(
            await configResponse.Content.ReadAsStreamAsync(TestContext.Current.CancellationToken),
            cancellationToken: TestContext.Current.CancellationToken);

        var config = configDocument.RootElement;
        Assert.Equal("PullInterval", config.GetProperty("syncMode").GetString());
        Assert.Equal(10, config.GetProperty("syncIntervalMinutes").GetInt32());
        var webhook = config.GetProperty("webhook");
        Assert.True(webhook.GetProperty("enabled").GetBoolean());
        Assert.Equal("GitLab", webhook.GetProperty("provider").GetString());
        Assert.Equal("GitLabSignedToken", webhook.GetProperty("authScheme").GetString());
        Assert.Equal("secret-123", webhook.GetProperty("secret").GetString());
    }

    private async Task<string?> GetPersistedWebhookJsonAsync(Guid repositoryId)
    {
        await using var scope = Services.CreateAsyncScope();
        var connectionFactory = scope.ServiceProvider.GetRequiredService<IDbConnectionFactory>();
        await using var connection = connectionFactory.Create();
        await connection.OpenAsync(TestContext.Current.CancellationToken);

        await using var command = connection.CreateCommand();
        command.CommandText = "SELECT Webhook::text FROM GitRepositories WHERE Id = @Id LIMIT 1";

        var id = command.CreateParameter();
        id.ParameterName = "@Id";
        id.Value = repositoryId;
        command.Parameters.Add(id);

        var result = await command.ExecuteScalarAsync(TestContext.Current.CancellationToken);
        return result?.ToString();
    }

    [Fact]
    public async Task Patch_GitRepository_When_Url_Changes_Deletes_Previous_Cache_And_Records_Update_Activity()
    {
        var patchJson = $$"""
        {
          "name": "OriginalName",
          "description": "Original description",
          "url": "https://github.com/citadel-p/citadel-api.git",
          "defaultBranch": "main",
          "webhook": { "enabled": false },
          "gitAccountId": "{{gitAccountId}}"
        }
        """;
        var content = new StringContent(patchJson, Encoding.UTF8, "application/merge-patch+json");

        var response = await Client.PatchAsync($"/api/v1/gitRepositories/{gitRepositoryId}", content, cancellationToken: TestContext.Current.CancellationToken);
        response.EnsureSuccessStatusCode();

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var activities = await uow.ActivityEventRepository.GetPagedAsync(
            gitRepositoryId,
            ActivityResourceType.GitRepository,
            ActivityEventType.GitRepoUpdated,
            1,
            10,
            TestContext.Current.CancellationToken);

        Assert.Single(activities.Items);
        Assert.True(_gitSyncChannel.Reader.TryRead(out var syncRequest));
        Assert.Equal(gitRepositoryId, syncRequest.RepoId);
    }

    [Fact]
    public async Task Patch_GitRepository_When_Name_Changes_Records_Rename_Activity()
    {
        var patchJson = $$"""
        {
          "id": "{{gitRepositoryId}}",
          "name": "RenamedRepo"
        }
        """;
        var content = new StringContent(patchJson, Encoding.UTF8, "application/json");

        var response = await Client.PostAsync("/api/v1/gitRepositories/rename", content, cancellationToken: TestContext.Current.CancellationToken);
        response.EnsureSuccessStatusCode();

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var gitRepository = await uow.GitRepositories.GetAsync(gitRepositoryId, TestContext.Current.CancellationToken);
        var activities = await uow.ActivityEventRepository.GetPagedAsync(
            gitRepositoryId,
            ActivityResourceType.GitRepository,
            ActivityEventType.GitRepoRenamed,
            1,
            10,
            TestContext.Current.CancellationToken);

        Assert.Equal("RenamedRepo", gitRepository?.Name);
        Assert.Single(activities.Items);
        Assert.False(_gitSyncChannel.Reader.TryRead(out _));
    }

    [Fact]
    public async Task Patch_GitRepository_Metadata_Should_Update_Description()
    {
        var patchJson = """
        {
          "description": "Metadata updated description"
        }
        """;
        var content = new StringContent(patchJson, Encoding.UTF8, "application/merge-patch+json");

        var response = await Client.PatchAsync($"/api/v1/gitRepositories/{gitRepositoryId}/_metadata", content, cancellationToken: TestContext.Current.CancellationToken);
        response.EnsureSuccessStatusCode();

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var gitRepository = await uow.GitRepositories.GetAsync(gitRepositoryId, TestContext.Current.CancellationToken);

        Assert.Equal("Metadata updated description", gitRepository?.Description);
        Assert.False(_gitSyncChannel.Reader.TryRead(out _));
    }

    [Fact]
    public async Task Patch_GitRepository_Remove_GitAccount_Should_Succeed()
    {
        var patchJson = """
        {
          "url": "https://gitlab.com/citadel-p/citadel.git",
          "defaultBranch": "main",
          "webhook": { "enabled": true },
          "gitAccountId": null
        }
        """;
        var content = new StringContent(patchJson, Encoding.UTF8, "application/merge-patch+json");

        var response = await Client.PatchAsync($"/api/v1/gitRepositories/{gitRepositoryId}", content, cancellationToken: TestContext.Current.CancellationToken);
        response.EnsureSuccessStatusCode();

        var responseBody = await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var gitRepository = await uow.GitRepositories.GetAsync(gitRepositoryId, TestContext.Current.CancellationToken);

        Assert.NotNull(gitRepository);
        Assert.Null(gitRepository.GitAccountId);
        await VerifyJson(responseBody);
    }

    [Fact]
    public async Task Patch_GitRepository_With_Invalid_Data_Should_Return_BadRequest()
    {
        var patchJson = $$"""
        {
          "url": "",
          "defaultBranch": "main",
          "webhook": { "enabled": true },
          "gitAccountId": "{{gitAccountId}}"
        }
        """;
        var content = new StringContent(patchJson, Encoding.UTF8, "application/merge-patch+json");

        var response = await Client.PatchAsync($"/api/v1/gitRepositories/{gitRepositoryId}", content, cancellationToken: TestContext.Current.CancellationToken);

        var responseBody = await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);
        await VerifyJson(responseBody);
    }

    [Fact]
    public async Task Patch_GitRepository_With_Unknown_GitAccount_Should_Return_NotFound()
    {
        var patchJson = $$"""
        {
          "name": "OriginalName",
          "description": "Original description",
          "url": "https://github.com/citadel-p/citadel.git",
          "defaultBranch": "main",
          "webhook": { "enabled": true },
          "gitAccountId": "{{Guid.NewGuid()}}"
        }
        """;
        var content = new StringContent(patchJson, Encoding.UTF8, "application/merge-patch+json");

        var response = await Client.PatchAsync($"/api/v1/gitRepositories/{gitRepositoryId}", content, cancellationToken: TestContext.Current.CancellationToken);

        var responseBody = await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);
        await VerifyJson(responseBody);
    }

    [Fact]
    public async Task Patch_GitRepository_With_Mismatched_GitAccount_Domain_Should_Return_BadRequest()
    {
        var patchJson = $$"""
        {
          "name": "OriginalName",
          "description": "Original description",
          "url": "https://gitlab.com/citadel-p/citadel.git",
          "defaultBranch": "main",
          "webhook": { "enabled": true },
          "gitAccountId": "{{gitAccountId}}"
        }
        """;
        var content = new StringContent(patchJson, Encoding.UTF8, "application/merge-patch+json");

        var response = await Client.PatchAsync($"/api/v1/gitRepositories/{gitRepositoryId}", content, cancellationToken: TestContext.Current.CancellationToken);

        var responseBody = await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);
        await VerifyJson(responseBody);
    }

    [Fact]
    public async Task Patch_GitRepository_With_Duplicate_Name_Should_Return_Conflict()
    {
        await using (var scope = Services.CreateAsyncScope())
        {
            var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            await uow.GitRepositories.AddAsync(new GitRepository(
                name: "OtherName",
                description: "Other repository",
                url: "https://github.com/citadel-p/other.git",
                defaultBranch: "main",
                gitAccountId: null,
                createdByActorId: Constants.SystemId
            ), TestContext.Current.CancellationToken);
            await uow.CommitAsync(TestContext.Current.CancellationToken);
        }

        var patchJson = $$"""
        {
          "id": "{{gitRepositoryId}}",
          "name": "OtherName"
        }
        """;
        var content = new StringContent(patchJson, Encoding.UTF8, "application/json");

        var response = await Client.PostAsync("/api/v1/gitRepositories/rename", content, cancellationToken: TestContext.Current.CancellationToken);

        var responseBody = await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);
        await VerifyJson(responseBody);
    }

    [Fact]
    public async Task Patch_GitRepository_Metadata_NonExistent_Should_Return_NotFound()
    {
        var patchJson = """
        {
          "description": "Updated metadata"
        }
        """;
        var content = new StringContent(patchJson, Encoding.UTF8, "application/merge-patch+json");

        var response = await Client.PatchAsync($"/api/v1/gitRepositories/{Guid.NewGuid()}/_metadata", content, cancellationToken: TestContext.Current.CancellationToken);

        var responseBody = await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);
        await VerifyJson(responseBody);
    }

    [Fact]
    public async Task Rename_NonExistent_GitRepository_Should_Return_NotFound()
    {
        var patchJson = $$"""
        {
          "id": "{{Guid.NewGuid()}}",
          "name": "DoesNotExist"
        }
        """;
        var content = new StringContent(patchJson, Encoding.UTF8, "application/json");

        var response = await Client.PostAsync("/api/v1/gitRepositories/rename", content, cancellationToken: TestContext.Current.CancellationToken);

        var responseBody = await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);
        await VerifyJson(responseBody);
    }
}
