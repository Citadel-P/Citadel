using Application.Services;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Alerts;
using Domain.Entities.Stacks;
using Microsoft.Extensions.DependencyInjection;
using Moq;

namespace Tests.Unit.Application.Features.Stacks;

public class ManualStackAutoUpdateTests
{
    [Fact]
    public void ParseServices_extracts_image_and_injected_service_hash()
    {
        var stackId = Guid.CreateVersion7();
        var releaseId = Guid.CreateVersion7();
        const string compose = """
        services:
          api:
            image: nginx:1.27
          worker:
            image: busybox
        """;

        var services = StackComposeParser.ParseServices(stackId, releaseId, compose);

        Assert.Equal(2, services.Count);
        Assert.Equal("nginx:1.27", services["api"].Image);
        Assert.False(string.IsNullOrWhiteSpace(services["api"].ExpectedConfigHash));
        Assert.Equal("busybox", services["worker"].Image);
    }

    [Fact]
    public async Task LoadManualStackChecks_includes_only_deployed_manual_stacks_with_update_policy_and_registry()
    {
        var registryId = Guid.CreateVersion7();
        var eligible = CreateStack(
            "eligible",
            StackUpdateBehavior.Notify,
            registryId,
            StackReleaseStatus.Healthy,
            """
            services:
              api:
                image: nginx:1.27
              pinned:
                image: redis@sha256:abc
            """);
        var disabled = CreateStack("disabled", StackUpdateBehavior.Disabled, registryId, StackReleaseStatus.Healthy);
        var stopped = CreateStack("stopped", StackUpdateBehavior.Notify, registryId, StackReleaseStatus.Stopped);
        var missingRegistry = CreateStack("missing-registry", StackUpdateBehavior.Notify, null, StackReleaseStatus.Healthy);

        var stacks = new Mock<IStackRepository>();
        stacks
            .Setup(x => x.GetAllAsync(It.IsAny<CancellationToken>()))
            .ReturnsAsync([eligible, disabled, stopped, missingRegistry]);

        var uow = new Mock<IUnitOfWork>();
        uow.Setup(x => x.Stacks).Returns(stacks.Object);

        var services = new ServiceCollection()
            .AddScoped(_ => uow.Object)
            .BuildServiceProvider();

        var scheduler = new ImageScanScheduler(services.GetRequiredService<IServiceScopeFactory>());

        var checks = await scheduler.LoadManualStackChecksAsync(CancellationToken.None);

        var check = Assert.Single(checks);
        Assert.Equal(eligible.Id, check.Stack.Id);
        Assert.Equal("api", check.ServiceName);
        Assert.Equal("nginx:1.27", check.ImageName);
    }

    [Fact]
    public void AlertRule_metadata_accepts_service_scoped_stack_update_events()
    {
        var updates = new[]
        {
            new StackImageUpdateItem("api", "nginx:1.27", "sha256:old", "sha256:new")
        };

        Assert.True(AlertTypeMetadata.IsValidInfo(
            AlertType.StackServiceAutoUpdated,
            new StackServiceAutoUpdatedAlertInfo("demo", updates)));
        Assert.True(AlertTypeMetadata.IsValidInfo(
            AlertType.StackServiceAutoDeployFailed,
            new StackServiceAutoDeployFailedAlertInfo("demo", ["api"], "failed")));
    }

    private static Stack CreateStack(
        string name,
        StackUpdateBehavior updateBehavior,
        Guid? registryId,
        StackReleaseStatus status,
        string compose = "services:\n  api:\n    image: nginx:latest\n")
    {
        var stack = Stack.Create(
            name,
            Guid.CreateVersion7(),
            StackSource.WebEditor,
            Guid.CreateVersion7(),
            new ManualStack(
                ComposeFile: compose,
                UpdateBehavior: updateBehavior,
                RegistryId: registryId));

        stack.PartialUpdate(status);
        return stack;
    }
}
