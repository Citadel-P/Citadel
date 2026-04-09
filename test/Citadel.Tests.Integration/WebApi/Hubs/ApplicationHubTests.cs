using Application.Mappers;
using Application.Services.Abstractions;
using Application.Services.SignalR;
using Domain.Contracts.Interfaces;
using Microsoft.AspNetCore.Http;
using Microsoft.AspNetCore.SignalR;
using Microsoft.AspNetCore.SignalR.Client;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.DependencyInjection.Extensions;
using Moq;
using System.Diagnostics;
using Tests.Integration.Helpers;
using WebApi.Routes.Endpoints.Resources.Containers;
using WebApi.Routes.Endpoints.Resources.Images;
using WebApi.Routes.Endpoints.Resources.Platforms;

namespace Tests.Integration.WebApi.Hubs;

public class ApplicationHubTests(PostgresTestFixture fixture) : IntegrationTestBase(fixture)
{
    private Guid platformId;
    private readonly Mock<IStreamSubscriptionResolver> resolverMock = new();
    private readonly Mock<IStreamGroupManager> groupManagerMock = new();
    
    private HubConnection? connection;

    protected override void ConfigureTestServices(IServiceCollection services)
    {
        services.RemoveAll<IStreamSubscriptionResolver>();
        services.AddSingleton(resolverMock.Object);
        // Reconfigure SignalR to add the testing filter
        services.PostConfigure<HubOptions>(options =>
        {
            options.AddFilter<TestHttpContextSyncFilter>();
        });

    }

    protected override async ValueTask SeedDbAsync(IUnitOfWork uow)
    {
        var platform = Fakes.GetDummyPlatform();
        var imageResults = Fakes.GetDummyImages();
        var dockerContainers = Fakes.GetDummyContainers(4);
        var images = new List<Domain.Entities.Image>();
        foreach (var img in imageResults)
        {
            images.Add(img.Map(platform.Id));
        }
        var containers = dockerContainers.Map(images, platform.Id);

        await uow.Platforms.AddAsync(platform, TestContext.Current.CancellationToken);
        await uow.Images.BulkUpsertAsync(images, TestContext.Current.CancellationToken);
        await uow.Containers.BulkUpsertAsync(containers, TestContext.Current.CancellationToken);

        await uow.CommitAsync(TestContext.Current.CancellationToken);

        platformId = platform.Id;
    }

    private async Task<HubConnection> CreateConnectionAsync()
    {
        if (connection != null)
        {
            await connection.DisposeAsync();
            connection = null;
        }

        var hubUrl = new Uri(Client.BaseAddress!, "/hubs/global");

        connection = new HubConnectionBuilder()
            .WithUrl(hubUrl, options =>
            {
                options.AccessTokenProvider = () => Task.FromResult(CreateJwtToken());
                options.HttpMessageHandlerFactory = _ => CreateServerHandler();
            })
            .Build();

        await connection.StartAsync();
        return connection;
    }

    [Fact]
    public async Task GetPlatforms_ReturnsSeededPlatform()
    {
        var conn = await CreateConnectionAsync();

        // Act
        var view = await conn.InvokeAsync<PlatformsView>("GetPlatforms", cancellationToken: TestContext.Current.CancellationToken);

        // Assert
        Assert.NotNull(view);
        Assert.Single(view.Platforms);
    }

    [Fact]
    public async Task GetContainers_ReturnsSeededContainers()
    {
        var conn = await CreateConnectionAsync();

        // Act
        var view = await conn.InvokeAsync<ContainersView>("GetContainers", platformId, cancellationToken: TestContext.Current.CancellationToken);

        // Assert
        Assert.NotNull(view);
        Assert.Equal(4, view.Containers.Count());
        Assert.Equal(3, view.Containers.Select(i => i.ImageView).Where(i => i is not null).Count());
    }

    [Fact]
    public async Task GetImages_ReturnsSeededImages()
    {
        var conn = await CreateConnectionAsync();

        // Act
        var view = await conn.InvokeAsync<ImagesView>("GetImages", platformId, cancellationToken: TestContext.Current.CancellationToken);

        // Assert
        Assert.NotNull(view);
        Assert.Equal(3, view.Images.Count());
    }

    [Fact]
    public async Task JoinGroup_AddsSubscriber_OnResolver()
    {
        // Arrange
        resolverMock.Setup(r => r.Resolve(It.IsAny<string>())).Returns(groupManagerMock.Object);

        var conn = await CreateConnectionAsync();
        var groupId = Guid.NewGuid().ToString();

        // Act
        await conn.InvokeAsync("JoinGroup", groupId, cancellationToken: TestContext.Current.CancellationToken);

        // Wait for AddSubscriber invocation (polling instead of Task.Delay)
        await WaitUntilAsync(() =>
            groupManagerMock.Invocations.Any(inv =>
                inv.Method.Name == nameof(IStreamGroupManager.AddSubscriber) &&
                inv.Arguments.Count > 0 &&
                (string?)inv.Arguments[0] == groupId),
            timeout: TimeSpan.FromSeconds(5));

        // Assert
        groupManagerMock.Verify(g => g.AddSubscriber(It.Is<string>(s => s == groupId), It.IsAny<string>()), Times.Once);
    }

    [Fact]
    public async Task LeaveGroup_RemovesSubscriber_And_OnDisconnect_RemovesConnection()
    {
        // Arrange
        var groupId = Guid.NewGuid().ToString();
        resolverMock.Setup(r => r.Resolve(groupId)).Returns(groupManagerMock.Object);

        var conn = await CreateConnectionAsync();

        // Join then leave
        await conn.InvokeAsync("JoinGroup", groupId, cancellationToken: TestContext.Current.CancellationToken);
        await WaitUntilAsync(() =>
            groupManagerMock.Invocations.Any(inv => inv.Method.Name == nameof(IStreamGroupManager.AddSubscriber) &&
                                                   inv.Arguments.Count > 0 &&
                                                   (string?)inv.Arguments[0] == groupId),
            timeout: TimeSpan.FromSeconds(5));

        await conn.InvokeAsync("LeaveGroup", groupId, cancellationToken: TestContext.Current.CancellationToken);

        // Wait for RemoveSubscriber invocation
        await WaitUntilAsync(() =>
            groupManagerMock.Invocations.Any(inv =>
                inv.Method.Name == nameof(IStreamGroupManager.RemoveSubscriber) &&
                inv.Arguments.Count > 0 &&
                (string?)inv.Arguments[0] == groupId),
            timeout: TimeSpan.FromSeconds(5));

        // Assert RemoveSubscriber called on leave
        groupManagerMock.Verify(g => g.RemoveSubscriber(groupId, It.IsAny<string>()), Times.Once);

        // Stop connection to trigger OnDisconnectedAsync and allow cleanup to run
        await conn.StopAsync(TestContext.Current.CancellationToken);

        groupManagerMock.Verify(g => g.RemoveConnection(It.IsAny<string>()), Times.Never);

        await conn.DisposeAsync();
    }

    // Poll/wait helper to replace Task.Delay-based timing
    private static async Task WaitUntilAsync(Func<bool> predicate, TimeSpan? timeout = null, TimeSpan? pollInterval = null)
    {
        var to = timeout ?? TimeSpan.FromSeconds(5);
        var poll = pollInterval ?? TimeSpan.FromMilliseconds(100);
        var sw = Stopwatch.StartNew();
        while (sw.Elapsed < to)
        {
            if (predicate()) return;
            await Task.Delay(poll);
        }
        throw new TimeoutException("Condition not satisfied within timeout");
    }
}

internal sealed class TestHttpContextSyncFilter : IHubFilter
{
    private readonly IHttpContextAccessor? _accessor;

    public TestHttpContextSyncFilter(IHttpContextAccessor? accessor = null)
    {
        _accessor = accessor;
    }

    public async ValueTask<object?> InvokeMethodAsync(
        HubInvocationContext context,
        Func<HubInvocationContext, ValueTask<object?>> next)
    {
        // Ensure the HttpContextAccessor is populated for this invocation
        if (_accessor is not null && context.Context.GetHttpContext() is { } httpCtx)
        {
            _accessor.HttpContext = httpCtx;
        }

        return await next(context);
    }
}