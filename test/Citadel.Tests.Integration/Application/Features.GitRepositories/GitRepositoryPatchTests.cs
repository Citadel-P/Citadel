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
using System.Threading.Channels;

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
          "webHookEnabled": true,
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
    public async Task Patch_GitRepository_When_Url_Changes_Deletes_Previous_Cache_And_Records_Update_Activity()
    {
        var patchJson = $$"""
        {
          "name": "OriginalName",
          "description": "Original description",
          "url": "https://github.com/citadel-p/citadel-api.git",
          "defaultBranch": "main",
          "webHookEnabled": false,
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
          "webHookEnabled": true,
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
          "webHookEnabled": true,
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
          "webHookEnabled": true,
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
          "webHookEnabled": true,
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
