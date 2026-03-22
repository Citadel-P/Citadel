using System.Text;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Git;
using Hosting.Common;
using Microsoft.Extensions.DependencyInjection;

namespace Tests.Integration.Application.Features.GitRepositories;

public class GitRepositoryPatchTests : IntegrationTestBase
{
    private Guid gitRepositoryId;
    private Guid gitAccountId;

    protected override async ValueTask SeedDbAsync(IUnitOfWork uow)
    {
        var gitAccount = new GitAccount(
            name: "GA-ORIGINAL",
            domain: "github.com",
            authType: GitAuthType.None,
            createdByActorId: Constants.SystemId,
            configuration: new NoAuthAccount());

        await uow.GitAccounts.AddAsync(gitAccount, TestContext.Current.CancellationToken);

        var gitRepository = new GitRepository(
            name: "OriginalName",
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
          "name": "UpdatedName",
          "url": "https://github.com/citadel-p/citadel-api.git",
          "defaultBranch": "develop",
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
        Assert.Equal("UpdatedName", gitRepository.Name);
        Assert.Equal("develop", gitRepository.DefaultBranch);
        await VerifyJson(responseBody);
    }

    [Fact]
    public async Task Patch_GitRepository_Remove_GitAccount_Should_Succeed()
    {
        var patchJson = """
        {
          "name": "OriginalName",
          "url": "https://gitlab.com/citadel-p/citadel.git",
          "defaultBranch": "main",
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
          "name": "",
          "url": "https://github.com/citadel-p/citadel.git",
          "defaultBranch": "main",
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
          "url": "https://github.com/citadel-p/citadel.git",
          "defaultBranch": "main",
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
          "url": "https://gitlab.com/citadel-p/citadel.git",
          "defaultBranch": "main",
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
                url: "https://github.com/citadel-p/other.git",
                defaultBranch: "main",
                gitAccountId: null,
                createdByActorId: Constants.SystemId
            ), TestContext.Current.CancellationToken);
            await uow.CommitAsync(TestContext.Current.CancellationToken);
        }

        var patchJson = $$"""
        {
          "name": "OtherName",
          "url": "https://github.com/citadel-p/citadel.git",
          "defaultBranch": "main",
          "gitAccountId": "{{gitAccountId}}"
        }
        """;
        var content = new StringContent(patchJson, Encoding.UTF8, "application/merge-patch+json");

        var response = await Client.PatchAsync($"/api/v1/gitRepositories/{gitRepositoryId}", content, cancellationToken: TestContext.Current.CancellationToken);

        var responseBody = await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);
        await VerifyJson(responseBody);
    }

    [Fact]
    public async Task Patch_NonExistent_GitRepository_Should_Return_NotFound()
    {
        var patchJson = """
        {
          "name": "DoesNotExist",
          "url": "https://github.com/citadel-p/citadel.git",
          "defaultBranch": "main",
          "gitAccountId": null
        }
        """;
        var content = new StringContent(patchJson, Encoding.UTF8, "application/merge-patch+json");

        var response = await Client.PatchAsync($"/api/v1/gitRepositories/{Guid.NewGuid()}", content, cancellationToken: TestContext.Current.CancellationToken);

        var responseBody = await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);
        await VerifyJson(responseBody);
    }
}
