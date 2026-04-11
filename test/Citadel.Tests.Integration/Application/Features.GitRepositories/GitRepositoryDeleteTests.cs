using System.Text;
using Application.Services;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Git;
using Hosting.Common;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.DependencyInjection.Extensions;
using Microsoft.Extensions.Hosting;
using Moq;

namespace Tests.Integration.Application.Features.GitRepositories;

public class GitRepositoryDeleteTests(PostgresTestFixture fixture) : IntegrationTestBase(fixture)
{
    private readonly Mock<IRepoCacheManager> _repoCacheManagerMock = new();
    private Guid gitRepositoryId;

    protected override void ConfigureTestServices(IServiceCollection services)
    {
        services.RemoveAll<IHostedService>();
        services.AddSingleton(_repoCacheManagerMock.Object);
    }

    protected override async ValueTask SeedDbAsync(IUnitOfWork uow)
    {
        var gitRepository = new GitRepository(
            name: "delete-me",
            description: "Delete me",
            url: "https://github.com/citadel-p/citadel.git",
            defaultBranch: "main",
            gitAccountId: null,
            createdByActorId: Constants.SystemId);

        await uow.GitRepositories.AddAsync(gitRepository, TestContext.Current.CancellationToken);
        await uow.CommitAsync(TestContext.Current.CancellationToken);
        gitRepositoryId = gitRepository.Id;
    }

    [Fact]
    public async Task Delete_GitRepository_ReturnsSuccess()
    {
        var content = $$"""
        {
            "ids": ["{{gitRepositoryId}}"]
        }
        """;

        var request = new HttpRequestMessage(HttpMethod.Delete, "/api/v1/gitRepositories")
        {
            Content = new StringContent(content, Encoding.UTF8, "application/json")
        };

        var response = await Client.SendAsync(request, TestContext.Current.CancellationToken);

        response.EnsureSuccessStatusCode();

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var gitRepositories = await uow.GitRepositories.GetAllAsync(TestContext.Current.CancellationToken);

        Assert.Empty(gitRepositories);
        Assert.Equal(string.Empty, await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken));
        _repoCacheManagerMock.Verify(x => x.DeleteCacheAsync(It.Is<GitRepository>(r => r.Id == gitRepositoryId), It.IsAny<CancellationToken>()), Times.Once);
    }

    [Fact]
    public async Task Delete_GitRepository_RecordsDeleteActivity()
    {
        var content = $$"""
        {
            "ids": ["{{gitRepositoryId}}"]
        }
        """;

        var request = new HttpRequestMessage(HttpMethod.Delete, "/api/v1/gitRepositories")
        {
            Content = new StringContent(content, Encoding.UTF8, "application/json")
        };

        var response = await Client.SendAsync(request, TestContext.Current.CancellationToken);

        response.EnsureSuccessStatusCode();

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var activities = await uow.ActivityEventRepository.GetPagedAsync(
            gitRepositoryId,
            ActivityResourceType.GitRepository,
            ActivityEventType.GitRepoDeleted,
            1,
            10,
            TestContext.Current.CancellationToken);

        Assert.Single(activities.Items);
    }

    [Fact]
    public async Task Delete_NonExistent_GitRepository_ReturnsNotFound()
    {
        var content = $$"""
        {
            "ids": ["{{Guid.NewGuid()}}"]
        }
        """;

        var request = new HttpRequestMessage(HttpMethod.Delete, "/api/v1/gitRepositories")
        {
            Content = new StringContent(content, Encoding.UTF8, "application/json")
        };

        var response = await Client.SendAsync(request, TestContext.Current.CancellationToken);

        var responseBody = await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken);
        await VerifyJson(responseBody);
    }
}
