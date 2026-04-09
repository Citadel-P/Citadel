using System.Text;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Git;
using Hosting.Common;
using Microsoft.Extensions.DependencyInjection;

namespace Tests.Integration.Application.Features.GitAccounts;

public class GitAccountPatchTests(PostgresTestFixture fixture) : IntegrationTestBase(fixture)
{
    private Guid gitAccountId;

    protected override async ValueTask SeedDbAsync(IUnitOfWork uow)
    {
        var gitAccount = new GitAccount(
            name: "OriginalName",
            domain: "github.com",
            transport: GitTransport.Https,
            authType: GitAuthType.Token,
            createdByActorId: Constants.SystemId,
            configuration: new TokenAuth("original-token"));

        await uow.GitAccounts.AddAsync(gitAccount, TestContext.Current.CancellationToken);
        await uow.CommitAsync(TestContext.Current.CancellationToken);

        gitAccountId = gitAccount.Id;
    }

    [Fact]
    public async Task Patch_GitAccount_Should_Apply_MergePatch()
    {
        var patchJson = """
        {
          "name": "UpdatedName",
          "domain": "github.com",
          "transport": "Https",
          "authType": "Token",
          "configuration": {
            "$type": "Token",
            "token": "original-token"
          }
        }
        """;
        var content = new StringContent(patchJson, Encoding.UTF8, "application/merge-patch+json");

        var response = await Client.PatchAsync($"/api/v1/gitAccounts/{gitAccountId}", content, cancellationToken: TestContext.Current.CancellationToken);
        response.EnsureSuccessStatusCode();

        var responseBody = await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var gitAccount = await uow.GitAccounts.GetAsync(gitAccountId, TestContext.Current.CancellationToken);

        Assert.NotNull(gitAccount);
        Assert.Equal("UpdatedName", gitAccount.Name);
        await VerifyJson(responseBody);
    }

    [Fact]
    public async Task Patch_GitAccount_Transport_And_AuthType_Should_Succeed()
    {
        var patchJson = """
        {
          "name": "OriginalName",
          "domain": "gitlab.com",
          "transport": "Ssh",
          "authType": "SshKey",
          "configuration": {
            "$type": "SshKey",
            "username": "git",
            "privateKey": "-----BEGIN OPENSSH PRIVATE KEY-----patched"
          }
        }
        """;
        var content = new StringContent(patchJson, Encoding.UTF8, "application/merge-patch+json");

        var response = await Client.PatchAsync($"/api/v1/gitAccounts/{gitAccountId}", content, cancellationToken: TestContext.Current.CancellationToken);
        response.EnsureSuccessStatusCode();

        var responseBody = await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var gitAccount = await uow.GitAccounts.GetAsync(gitAccountId, TestContext.Current.CancellationToken);

        Assert.NotNull(gitAccount);
        Assert.Equal("gitlab.com", gitAccount.Domain);
        Assert.Equal(GitTransport.Ssh, gitAccount.Transport);
        Assert.Equal(GitAuthType.SshKey, gitAccount.AuthType);
        await VerifyJson(responseBody);
    }

    [Fact]
    public async Task Patch_GitAccount_With_Invalid_Data_Should_Return_BadRequest()
    {
        var patchJson = """
        {
          "name": "",
          "domain": "github.com",
          "transport": "Https",
          "authType": "Token",
          "configuration": {
            "$type": "Token",
            "token": "original-token"
          }
        }
        """;
        var content = new StringContent(patchJson, Encoding.UTF8, "application/merge-patch+json");

        var response = await Client.PatchAsync($"/api/v1/gitAccounts/{gitAccountId}", content, cancellationToken: TestContext.Current.CancellationToken);

        var responseBody = await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);
        await VerifyJson(responseBody);
    }

    [Fact]
    public async Task Patch_GitAccount_With_Mismatched_Configuration_Should_Return_BadRequest()
    {
        var patchJson = """
        {
          "name": "OriginalName",
          "domain": "github.com",
          "transport": "Https",
          "authType": "Basic",
          "configuration": {
            "$type": "SshKey",
            "username": "git",
            "privateKey": "-----BEGIN OPENSSH PRIVATE KEY-----patched"
          }
        }
        """;
        var content = new StringContent(patchJson, Encoding.UTF8, "application/merge-patch+json");

        var response = await Client.PatchAsync($"/api/v1/gitAccounts/{gitAccountId}", content, cancellationToken: TestContext.Current.CancellationToken);

        var responseBody = await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);
        await VerifyJson(responseBody);
    }

    [Fact]
    public async Task Patch_GitAccount_With_Duplicate_Name_Should_Return_Conflict()
    {
        await using (var scope = Services.CreateAsyncScope())
        {
            var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            await uow.GitAccounts.AddAsync(new GitAccount(
                name: "OtherName",
                domain: "github.com",
                transport: GitTransport.Https,
                authType: GitAuthType.Basic,
                createdByActorId: Constants.SystemId,
                configuration: new BasicAuth("dummy-user", "dummy-password123")
            ), TestContext.Current.CancellationToken);
            await uow.CommitAsync(TestContext.Current.CancellationToken);
        }

        var patchJson = """
        {
          "name": "OtherName",
          "domain": "github.com",
          "transport": "Https",
          "authType": "Token",
          "configuration": {
            "$type": "Token",
            "token": "original-token"
          }
        }
        """;
        var content = new StringContent(patchJson, Encoding.UTF8, "application/merge-patch+json");

        var response = await Client.PatchAsync($"/api/v1/gitAccounts/{gitAccountId}", content, cancellationToken: TestContext.Current.CancellationToken);

        var responseBody = await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);
        await VerifyJson(responseBody);
    }

    [Fact]
    public async Task Patch_NonExistent_GitAccount_Should_Return_NotFound()
    {
        var nonExistentId = Guid.NewGuid();
        var patchJson = """
        {
          "name": "DoesNotExist",
          "domain": "github.com",
          "transport": "Https",
          "authType": "Basic",
          "configuration": {
            "$type": "Basic",
            "username": "dummy-user",
            "password": "dummy-password123"
          }
        }
        """;
        var content = new StringContent(patchJson, Encoding.UTF8, "application/merge-patch+json");

        var response = await Client.PatchAsync($"/api/v1/gitAccounts/{nonExistentId}", content, cancellationToken: TestContext.Current.CancellationToken);

        var responseBody = await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);
        await VerifyJson(responseBody);
    }
}
