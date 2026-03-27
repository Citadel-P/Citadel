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
    public async Task Create_BasicGitAccount_ReturnsSuccess()
    {
        var createJson = """
        {
          "name": "GA-NEW",
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
        var content = new StringContent(createJson, Encoding.UTF8, "application/json");

        var response = await Client.PostAsync("/api/v1/gitAccounts", content, cancellationToken: TestContext.Current.CancellationToken);

        response.EnsureSuccessStatusCode();
        var responseBody = await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var gitAccounts = await uow.GitAccounts.GetAllAsync(TestContext.Current.CancellationToken);

        Assert.Contains(gitAccounts, x => x.Name == "GA-NEW" && x.Transport == GitTransport.Https && x.AuthType == GitAuthType.Basic);
        await VerifyJson(responseBody);
    }

    [Fact]
    public async Task Create_TokenGitAccount_ReturnsSuccess()
    {
        var createJson = """
        {
          "name": "GA-NEW",
          "domain": "github.com",
          "transport": "Https",
          "authType": "Token",
          "configuration": {
            "$type": "Token",
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

        Assert.Contains(gitAccounts, x => x.Name == "GA-NEW" && x.Transport == GitTransport.Https && x.AuthType == GitAuthType.Token);
        await VerifyJson(responseBody);
    }

    [Fact]
    public async Task Create_SshKeyGitAccount_ReturnsSuccess()
    {
        var createJson = """
        {
          "name": "GA-NEW",
          "domain": "github.com",
          "transport": "Ssh",
          "authType": "SshKey",
          "configuration": {
            "$type": "SshKey",
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

        Assert.Contains(gitAccounts, x => x.Name == "GA-NEW" && x.Transport == GitTransport.Ssh && x.AuthType == GitAuthType.SshKey);
        await VerifyJson(responseBody);
    }

    [Fact]
    public async Task Create_GitAccount_With_Empty_Name_Returns_BadRequest()
    {
        var createJson = """
        {
          "name": "",
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
          "transport": "Https",
          "authType": "Token",
          "configuration": {
            "$type": "Token",
            "token": ""
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
                transport: GitTransport.Https,
                authType: GitAuthType.Basic,
                createdByActorId: Constants.SystemId,
                configuration: new BasicAuth("dummy-user", "dummy-password123")
            ), TestContext.Current.CancellationToken);
            await uow.CommitAsync(TestContext.Current.CancellationToken);
        }

        var createJson = """
        {
          "name": "GA-NEW",
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
          "transport": "Https",
          "authType": "Basic",
          "configuration": {
            "$type": "SshKey",
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
