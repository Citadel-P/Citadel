using Application.Features.Automation.Commands;
using Application.Features.Automation.Models;
using Domain.Contracts.Resources.Automation;
using Hosting.Common.Abstraction;
using Hosting.Common.ErrorTypes;
using Hosting.Common.Pipelines;
using Hosting.Common.Pipelines.Interfaces;
using LightResults;
using Microsoft.AspNetCore.Http;
using Moq;

namespace Tests.Unit.Application.Services;

public sealed class StreamPermissionBehaviorTests
{
    [Fact]
    public async Task Denied_Permission_Sets_Forbidden_Response_Instead_Of_Empty_Success()
    {
        var userId = Guid.CreateVersion7();
        var user = new Mock<IUserContext>();
        user.SetupGet(x => x.UserId).Returns(userId);
        user.SetupGet(x => x.IsAuthenticated).Returns(true);
        user.SetupGet(x => x.IsAdmin).Returns(false);

        var userContext = new Mock<IUserContextAccessor>();
        userContext.SetupGet(x => x.Current).Returns(user.Object);

        var permissionService = new Mock<IPermissionService>();
        permissionService
            .Setup(x => x.EnforceAsync(
                It.IsAny<TestAutomationAction>(),
                userId,
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(Result.Failure(new ForbiddenError("Missing Execute permission.")));

        var httpContext = new DefaultHttpContext();
        var behavior = new StreamPermissionBehavior<TestAutomationAction, AutomationActionRunStreamItem>(
            userContext.Object,
            permissionService.Object,
            new HttpContextAccessor { HttpContext = httpContext });
        var nextCalled = false;
        var command = new TestAutomationAction(
            Guid.CreateVersion7(),
            new TestAutomationActionInputModel("console.log('test')", null, null, 30, null));

        await foreach (var _ in behavior.Handle(command, Next, TestContext.Current.CancellationToken))
        {
        }

        Assert.False(nextCalled);
        Assert.Equal(StatusCodes.Status403Forbidden, httpContext.Response.StatusCode);

        async IAsyncEnumerable<AutomationActionRunStreamItem> Next(
            TestAutomationAction _,
            CancellationToken cancellationToken)
        {
            nextCalled = true;
            await Task.CompletedTask;
            cancellationToken.ThrowIfCancellationRequested();
            yield break;
        }
    }
}
