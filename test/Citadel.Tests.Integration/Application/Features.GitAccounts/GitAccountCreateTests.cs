using System.Text;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Git;
using Hosting.Common;
using Microsoft.Extensions.DependencyInjection;

namespace Tests.Integration.Application.Features.GitAccounts;

public class GitAccountCreateTests : IntegrationTestBase
{
    [Fact]
    public async Task Create_NoAuthGitAccount_ReturnsSuccess()
    {
        var createJson = """
        {
          "name": "GA-NEW",
          "domain": "github.com",
          "authType": "None",
          "configuration": {
            "$type": "None"
          }
        }
        """;
        var content = new StringContent(createJson, Encoding.UTF8, "application/json");

        var response = await Client.PostAsync("/api/v1/gitAccounts", content, cancellationToken: TestContext.Current.CancellationToken);

        response.EnsureSuccessStatusCode();
        var responseBody = await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var gitAccounts = await uow.GitAccounts.GetAllAsync(TestContext.Current.CancellationToken);

        Assert.Contains(gitAccounts, x => x.Name == "GA-NEW" && x.AuthType == GitAuthType.None);
        await VerifyJson(responseBody);
    }

    [Fact]
    public async Task Create_HttpsGitAccount_ReturnsSuccess()
    {
        var createJson = """
        {
          "name": "GA-NEW",
          "domain": "github.com",
          "authType": "Https",
          "configuration": {
            "$type": "Https",
            "authEnabled": true,
            "username": "dummy-user",
            "token": "dummy-token123"
          }
        }
        """;
        var content = new StringContent(createJson, Encoding.UTF8, "application/json");

        var response = await Client.PostAsync("/api/v1/gitAccounts", content, cancellationToken: TestContext.Current.CancellationToken);

        response.EnsureSuccessStatusCode();
        var responseBody = await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var gitAccounts = await uow.GitAccounts.GetAllAsync(TestContext.Current.CancellationToken);

        Assert.Contains(gitAccounts, x => x.Name == "GA-NEW" && x.AuthType == GitAuthType.Https);
        await VerifyJson(responseBody);
    }

    [Fact]
    public async Task Create_SshGitAccount_ReturnsSuccess()
    {
        var createJson = """
        {
          "name": "GA-NEW",
          "domain": "github.com",
          "authType": "Ssh",
          "configuration": {
            "$type": "Ssh",
            "username": "git",
            "privateKey": "-----BEGIN OPENSSH PRIVATE KEY-----dummy"
          }
        }
        """;
        var content = new StringContent(createJson, Encoding.UTF8, "application/json");

        var response = await Client.PostAsync("/api/v1/gitAccounts", content, cancellationToken: TestContext.Current.CancellationToken);

        response.EnsureSuccessStatusCode();
        var responseBody = await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var gitAccounts = await uow.GitAccounts.GetAllAsync(TestContext.Current.CancellationToken);

        Assert.Contains(gitAccounts, x => x.Name == "GA-NEW" && x.AuthType == GitAuthType.Ssh);
        await VerifyJson(responseBody);
    }

    [Fact]
    public async Task Create_GitAccount_With_Empty_Name_Returns_BadRequest()
    {
        var createJson = """
        {
          "name": "",
          "domain": "github.com",
          "authType": "None",
          "configuration": {
            "$type": "None"
          }
        }
        """;
        var content = new StringContent(createJson, Encoding.UTF8, "application/json");

        var response = await Client.PostAsync("/api/v1/gitAccounts", content, cancellationToken: TestContext.Current.CancellationToken);

        var responseBody = await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);
        Assert.Equal(System.Net.HttpStatusCode.BadRequest, response.StatusCode);
        await VerifyJson(responseBody);
    }

    [Fact]
    public async Task Create_GitAccount_With_Invalid_Configuration_Returns_BadRequest()
    {
        var createJson = """
        {
          "name": "GA-NEW",
          "domain": "github.com",
          "authType": "Https",
          "configuration": {
            "$type": "Https",
            "authEnabled": true,
            "username": "",
            "token": null
          }
        }
        """;
        var content = new StringContent(createJson, Encoding.UTF8, "application/json");

        var response = await Client.PostAsync("/api/v1/gitAccounts", content, cancellationToken: TestContext.Current.CancellationToken);

        var responseBody = await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);
        Assert.Equal(System.Net.HttpStatusCode.BadRequest, response.StatusCode);
        await VerifyJson(responseBody);
    }

    [Fact]
    public async Task Create_GitAccount_With_Duplicate_Name_Returns_Conflict()
    {
        await using (var scope = Services.CreateAsyncScope())
        {
            var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            await uow.GitAccounts.AddAsync(new GitAccount(
                name: "GA-NEW",
                domain: "github.com",
                authType: GitAuthType.None,
                createdByActorId: Constants.SystemId,
                configuration: new NoAuthAccount()
            ), TestContext.Current.CancellationToken);
            await uow.CommitAsync(TestContext.Current.CancellationToken);
        }

        var createJson = """
        {
          "name": "GA-NEW",
          "domain": "github.com",
          "authType": "None",
          "configuration": {
            "$type": "None"
          }
        }
        """;
        var content = new StringContent(createJson, Encoding.UTF8, "application/json");

        var response = await Client.PostAsync("/api/v1/gitAccounts", content, cancellationToken: TestContext.Current.CancellationToken);

        var responseBody = await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);
        Assert.Equal(System.Net.HttpStatusCode.Conflict, response.StatusCode);
        await VerifyJson(responseBody);
    }

    [Fact]
    public async Task Create_GitAccount_With_Mismatched_Configuration_Returns_BadRequest()
    {
        var createJson = """
        {
          "name": "GA-NEW",
          "domain": "github.com",
          "authType": "None",
          "configuration": {
            "$type": "Ssh",
            "username": "git",
            "privateKey": "-----BEGIN OPENSSH PRIVATE KEY-----dummy"
          }
        }
        """;
        var content = new StringContent(createJson, Encoding.UTF8, "application/json");

        var response = await Client.PostAsync("/api/v1/gitAccounts", content, cancellationToken: TestContext.Current.CancellationToken);

        var responseBody = await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);
        Assert.Equal(System.Net.HttpStatusCode.BadRequest, response.StatusCode);
        await VerifyJson(responseBody);
    }
}
