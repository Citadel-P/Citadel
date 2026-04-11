using System.Text;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Git;
using Hosting.Common;
using Microsoft.Extensions.DependencyInjection;

namespace Tests.Integration.Application.Features.GitAccounts;

public class GitAccountDeleteTests(PostgresTestFixture fixture) : IntegrationTestBase(fixture)
{
    private Guid gitAccountId;

    protected override async ValueTask SeedDbAsync(IUnitOfWork uow)
    {
        var gitAccount = new GitAccount(
            name: "delete-me",
            domain: "github.com",
            transport: GitTransport.Https,
            authType: GitAuthType.Basic,
            createdByActorId: Constants.SystemId,
            configuration: new BasicAuth("dummy-user", "dummy-password123"));

        await uow.GitAccounts.AddAsync(gitAccount, TestContext.Current.CancellationToken);
        await uow.CommitAsync(TestContext.Current.CancellationToken);
        gitAccountId = gitAccount.Id;
    }

    [Fact]
    public async Task Delete_GitAccount_ReturnsSuccess()
    {
        var content = $$"""
        {
            "ids": ["{{gitAccountId}}"]
        }
        """;

        var request = new HttpRequestMessage(HttpMethod.Delete, "/api/v1/gitAccounts")
        {
            Content = new StringContent(content, Encoding.UTF8, "application/json")
        };

        var response = await Client.SendAsync(request, TestContext.Current.CancellationToken);

        response.EnsureSuccessStatusCode();

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var gitAccounts = await uow.GitAccounts.GetAllAsync(TestContext.Current.CancellationToken);

        Assert.Empty(gitAccounts);
    }

    [Fact]
    public async Task Delete_NonExistent_GitAccount_ReturnsNotFound()
    {
        var content = $$"""
        {
            "ids": ["{{Guid.NewGuid()}}"]
        }
        """;

        var request = new HttpRequestMessage(HttpMethod.Delete, "/api/v1/gitAccounts")
        {
            Content = new StringContent(content, Encoding.UTF8, "application/json")
        };

        var response = await Client.SendAsync(request, TestContext.Current.CancellationToken);

        var responseBody = await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);
        await VerifyJson(responseBody);
    }
}
