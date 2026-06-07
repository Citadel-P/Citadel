using Domain;
using Domain.Contracts.Interfaces;
using Hosting.Common;
using Microsoft.Extensions.DependencyInjection;
using System.Net;
using System.Text;
using Tests.Integration.Helpers;

namespace Tests.Integration.Application.Features.Stacks;

public class StackCreateTests(PostgresTestFixture fixture) : IntegrationTestBase(fixture)
{
    private Guid? platformId;

    protected override async ValueTask SeedDbAsync(IUnitOfWork uow)
    {
        var platform = Fakes.GetDummyPlatform();
        await uow.Platforms.AddAsync(platform, TestContext.Current.CancellationToken);
        await uow.CommitAsync(TestContext.Current.CancellationToken);
        platformId = platform.Id;
    }

    [Fact]
    public async Task Create_ManualStack_CreatesStackAndInitialRelease()
    {
        var createJson = $$"""
            {
                "name":"stack-1",
                "platformId":"{{platformId}}",
                "stackSource":"WebEditor",
                "spec":{
                    "$type":"WebEditor",
                    "composeFile":"docker-compose.yml"
                }
            }
            """;

        var content = new StringContent(createJson, Encoding.UTF8, "application/json");

        var response = await Client.PostAsync("/api/v1/stacks", content, cancellationToken: TestContext.Current.CancellationToken);

        response.EnsureSuccessStatusCode();

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var stacks = await uow.Stacks.GetInfoAsync(TestContext.Current.CancellationToken);
        var createdStack = Assert.Single(stacks);
        var releases = await uow.Stacks.GetReleasesByStackIdAsync(createdStack.Id, TestContext.Current.CancellationToken);
        var initialRelease = Assert.Single(releases);

        Assert.Equal("stack-1", createdStack.Name);
        Assert.Equal(initialRelease.Id, createdStack.CurrentStackReleaseId);
        Assert.Equal("1", initialRelease.Version);
    }

    [Fact]
    public async Task Create_Stack_With_Duplicate_Name_Returns_Conflict()
    {
        await using (var scope = Services.CreateAsyncScope())
        {
            var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            var stack = Domain.Entities.Stacks.Stack.Create(
                "stack-1",
                Constants.SystemId,
                StackSource.WebEditor,
                platformId!.Value,
                new Domain.Entities.Stacks.ManualStack("docker-compose.yml", StackUpdateBehavior.Disabled));

            await uow.Stacks.AddAsync(stack, TestContext.Current.CancellationToken);
            await uow.CommitAsync(TestContext.Current.CancellationToken);
        }

        var createJson = $$"""
            {
                "name":"stack-1",
                "platformId":"{{platformId}}",
                "stackSource":"WebEditor",
                "spec":{
                    "$type":"WebEditor",
                    "composeFile":"docker-compose.yml"
                }
            }
            """;

        var content = new StringContent(createJson, Encoding.UTF8, "application/json");

        var response = await Client.PostAsync("/api/v1/stacks", content, cancellationToken: TestContext.Current.CancellationToken);

        Assert.Equal(HttpStatusCode.Conflict, response.StatusCode);
    }

    [Fact]
    public async Task Create_Stack_With_Unknown_Platform_Returns_NotFound()
    {
        var createJson = $$"""
            {
                "name":"stack-1",
                "platformId":"{{Guid.NewGuid()}}",
                "stackSource":"WebEditor",
                "spec":{
                    "$type":"WebEditor",
                    "composeFile":"docker-compose.yml"
                }
            }
            """;

        var content = new StringContent(createJson, Encoding.UTF8, "application/json");

        var response = await Client.PostAsync("/api/v1/stacks", content, cancellationToken: TestContext.Current.CancellationToken);

        Assert.Equal(HttpStatusCode.NotFound, response.StatusCode);
    }
}