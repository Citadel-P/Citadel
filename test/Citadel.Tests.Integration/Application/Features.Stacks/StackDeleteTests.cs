using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Stacks;
using Hosting.Common;
using Microsoft.Extensions.DependencyInjection;
using System.Net;
using System.Text;
using Tests.Integration.Helpers;

namespace Tests.Integration.Application.Features.Stacks;

public class StackDeleteTests(PostgresTestFixture fixture) : IntegrationTestBase(fixture)
{
    private Guid stackId;

    protected override async ValueTask SeedDbAsync(IUnitOfWork uow)
    {
        var platform = Fakes.GetDummyPlatform();
        await uow.Platforms.AddAsync(platform, TestContext.Current.CancellationToken);

        var stack = Stack.Create(
            "delete-me",
            Constants.SystemId,
            StackSource.Manual,
            platform.Id,
            new ManualStack("docker-compose.yml", StackUpdateBehavior.Disabled));

        await uow.Stacks.AddAsync(stack, TestContext.Current.CancellationToken);
        await uow.CommitAsync(TestContext.Current.CancellationToken);
        stackId = stack.Id;
    }

    [Fact]
    public async Task Delete_Stack_ReturnsSuccess()
    {
        var content = $$"""
        ["{{stackId}}"]
        """;

        var request = new HttpRequestMessage(HttpMethod.Delete, "/api/v1/stacks")
        {
            Content = new StringContent(content, Encoding.UTF8, "application/json")
        };

        var response = await Client.SendAsync(request, TestContext.Current.CancellationToken);

        response.EnsureSuccessStatusCode();

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var stacks = await uow.Stacks.GetInfoAsync(TestContext.Current.CancellationToken);

        Assert.Empty(stacks);
        Assert.Equal(string.Empty, await response.Content.ReadAsStringAsync(TestContext.Current.CancellationToken));
    }

    [Fact]
    public async Task Delete_NonExistent_Stack_ReturnsNotFound()
    {
        var content = $$"""
        ["{{Guid.NewGuid()}}"]
        """;

        var request = new HttpRequestMessage(HttpMethod.Delete, "/api/v1/stacks")
        {
            Content = new StringContent(content, Encoding.UTF8, "application/json")
        };

        var response = await Client.SendAsync(request, TestContext.Current.CancellationToken);

        Assert.Equal(HttpStatusCode.NotFound, response.StatusCode);
    }
}
