using System.Text;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Git;
using Hosting.Common;
using Microsoft.Extensions.DependencyInjection;

namespace Tests.Integration.Application.Features.GitRepositories;

public class GitRepositoryCreateTests : IntegrationTestBase
{
    [Fact]
    public async Task Create_GitRepository_Without_GitAccount_ReturnsSuccess()
    {
        var createJson = """
        {
          "name": "GR-NEW",
          "description": "A git repository",
          "url": "https://github.com/citadel-p/citadel.git",
          "defaultBranch": "main",
          "status": "Valid",
          "gitAccountId": null
        }
        """;
        var content = new StringContent(createJson, Encoding.UTF8, "application/json");

        var response = await Client.PostAsync("/api/v1/gitRepositories", content, cancellationToken: TestContext.Current.CancellationToken);

        response.EnsureSuccessStatusCode();
        var responseBody = await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var gitRepositories = await uow.GitRepositories.GetAllAsync(TestContext.Current.CancellationToken);

        Assert.Contains(gitRepositories, x => x.Name == "GR-NEW" && x.DefaultBranch == "main" && x.Status == GitReposStatus.Valid);
        await VerifyJson(responseBody);
    }

    [Fact]
    public async Task Create_GitRepository_With_Linked_GitAccount_ReturnsSuccess()
    {
        Guid gitAccountId;
        await using (var scope = Services.CreateAsyncScope())
        {
            var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            var gitAccount = new GitAccount(
                name: "GA-NEW",
                domain: "github.com",
                authType: GitAuthType.None,
                createdByActorId: Constants.SystemId,
                configuration: new NoAuthAccount());

            await uow.GitAccounts.AddAsync(gitAccount, TestContext.Current.CancellationToken);
            await uow.CommitAsync(TestContext.Current.CancellationToken);
            gitAccountId = gitAccount.Id;
        }

        var createJson = $$"""
        {
          "name": "GR-NEW",
          "description": "A linked repository",
          "url": "https://github.com/citadel-p/citadel.git",
          "defaultBranch": "main",
          "status": "Valid",
          "gitAccountId": "{{gitAccountId}}"
        }
        """;
        var content = new StringContent(createJson, Encoding.UTF8, "application/json");

        var response = await Client.PostAsync("/api/v1/gitRepositories", content, cancellationToken: TestContext.Current.CancellationToken);

        response.EnsureSuccessStatusCode();
        var responseBody = await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);

        await using var scope2 = Services.CreateAsyncScope();
        var uow2 = scope2.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var gitRepositories = await uow2.GitRepositories.GetAllAsync(TestContext.Current.CancellationToken);

        Assert.Contains(gitRepositories, x => x.Name == "GR-NEW" && x.GitAccountId == gitAccountId);
        await VerifyJson(responseBody);
    }

    [Fact]
    public async Task Create_GitRepository_With_Empty_Name_Returns_BadRequest()
    {
        var createJson = """
        {
          "name": "",
          "description": "A git repository",
          "url": "https://github.com/citadel-p/citadel.git",
          "defaultBranch": "main",
          "status":  "Valid",
          "gitAccountId": null
        }
        """;
        var content = new StringContent(createJson, Encoding.UTF8, "application/json");

        var response = await Client.PostAsync("/api/v1/gitRepositories", content, cancellationToken: TestContext.Current.CancellationToken);

        var responseBody = await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);
        Assert.Equal(System.Net.HttpStatusCode.BadRequest, response.StatusCode);
        await VerifyJson(responseBody);
    }

    [Fact]
    public async Task Create_GitRepository_With_Empty_DefaultBranch_Returns_BadRequest()
    {
        var createJson = """
        {
          "name": "GR-NEW",
          "description": "A git repository",
          "url": "https://github.com/citadel-p/citadel.git",
          "defaultBranch": "",
          "status": "Valid",
          "gitAccountId": null
        }
        """;
        var content = new StringContent(createJson, Encoding.UTF8, "application/json");

        var response = await Client.PostAsync("/api/v1/gitRepositories", content, cancellationToken: TestContext.Current.CancellationToken);

        var responseBody = await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);
        Assert.Equal(System.Net.HttpStatusCode.BadRequest, response.StatusCode);
        await VerifyJson(responseBody);
    }

    [Fact]
    public async Task Create_GitRepository_With_Duplicate_Name_Returns_Conflict()
    {
        await using (var scope = Services.CreateAsyncScope())
        {
            var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            await uow.GitRepositories.AddAsync(new GitRepository(
                name: "GR-NEW",
                description: "Existing repository",
                url: "https://github.com/citadel-p/citadel.git",
                defaultBranch: "main",
                status: GitReposStatus.Valid,
                gitAccountId: null,
                createdByActorId: Constants.SystemId
            ), TestContext.Current.CancellationToken);
            await uow.CommitAsync(TestContext.Current.CancellationToken);
        }

        var createJson = """
        {
          "name": "GR-NEW",
          "description": "Another repository",
          "url": "https://github.com/citadel-p/other.git",
          "defaultBranch": "main",
          "status": "Valid",
          "gitAccountId": null
        }
        """;
        var content = new StringContent(createJson, Encoding.UTF8, "application/json");

        var response = await Client.PostAsync("/api/v1/gitRepositories", content, cancellationToken: TestContext.Current.CancellationToken);

        var responseBody = await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);
        Assert.Equal(System.Net.HttpStatusCode.Conflict, response.StatusCode);
        await VerifyJson(responseBody);
    }

    [Fact]
    public async Task Create_GitRepository_With_Unknown_GitAccount_Returns_NotFound()
    {
        var createJson = $$"""
        {
          "name": "GR-NEW",
          "description": "A git repository",
          "url": "https://github.com/citadel-p/citadel.git",
          "defaultBranch": "main",
          "status": "Valid",
          "gitAccountId": "{{Guid.NewGuid()}}"
        }
        """;
        var content = new StringContent(createJson, Encoding.UTF8, "application/json");

        var response = await Client.PostAsync("/api/v1/gitRepositories", content, cancellationToken: TestContext.Current.CancellationToken);

        var responseBody = await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);
        Assert.Equal(System.Net.HttpStatusCode.NotFound, response.StatusCode);
        await VerifyJson(responseBody);
    }

    [Fact]
    public async Task Create_GitRepository_With_Mismatched_GitAccount_Domain_Returns_BadRequest()
    {
        Guid gitAccountId;
        await using (var scope = Services.CreateAsyncScope())
        {
            var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            var gitAccount = new GitAccount(
                name: "GA-NEW",
                domain: "github.com",
                authType: GitAuthType.None,
                createdByActorId: Constants.SystemId,
                configuration: new NoAuthAccount());

            await uow.GitAccounts.AddAsync(gitAccount, TestContext.Current.CancellationToken);
            await uow.CommitAsync(TestContext.Current.CancellationToken);
            gitAccountId = gitAccount.Id;
        }

        var createJson = $$"""
        {
          "name": "GR-NEW",
          "description": "A git repository",
          "url": "https://gitlab.com/citadel-p/citadel.git",
          "defaultBranch": "main",
          "status": "Valid",
          "gitAccountId": "{{gitAccountId}}"
        }
        """;
        var content = new StringContent(createJson, Encoding.UTF8, "application/json");

        var response = await Client.PostAsync("/api/v1/gitRepositories", content, cancellationToken: TestContext.Current.CancellationToken);

        var responseBody = await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);
        Assert.Equal(System.Net.HttpStatusCode.BadRequest, response.StatusCode);
        await VerifyJson(responseBody);
    }
}
