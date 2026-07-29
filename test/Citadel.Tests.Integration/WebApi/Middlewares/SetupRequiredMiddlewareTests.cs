using Application.Services.Identity;
using Microsoft.AspNetCore.Http;
using Microsoft.Extensions.DependencyInjection;
using WebApi.Middlewares;

namespace Tests.Integration.WebApi.Middlewares;

public sealed class SetupRequiredMiddlewareTests
{
    [Fact]
    public async Task InvokeAsync_WithCachedState_ShouldNotResolveUnitOfWork()
    {
        var nextCalled = false;
        var cache = new SetupStateCache();
        cache.SetRequiresSetup(false);
        var middleware = new SetupRequiredMiddleware(
            _ =>
            {
                nextCalled = true;
                return Task.CompletedTask;
            },
            cache);
        var context = new DefaultHttpContext
        {
            RequestServices = new ServiceCollection().BuildServiceProvider()
        };
        context.Request.Path = "/api/v1/platforms";

        await middleware.InvokeAsync(context);

        Assert.True(nextCalled);
    }
}
