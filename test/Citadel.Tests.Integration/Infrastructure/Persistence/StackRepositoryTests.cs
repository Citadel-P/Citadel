using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Platforms;
using Domain.Entities.Stacks;
using Hosting.Common;
using Microsoft.Extensions.DependencyInjection;

namespace Tests.Integration.Infrastructure.Persistence;

public sealed class StackRepositoryTests(PostgresTestFixture fixture) : IntegrationTestBase(fixture)
{
    [Fact]
    public async Task ReplaceReleaseVolumeBindingsAsync_ShouldReplaceExistingBindingWithSameReleaseAndVolumeName()
    {
        var cancellationToken = TestContext.Current.CancellationToken;
        var actorId = Constants.SystemId;
        var suffix = Guid.NewGuid().ToString("N")[..8];

        Guid releaseId;
        Guid platformId;

        await using (var scope = Services.CreateAsyncScope())
        {
            var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            var platform = CreatePlatform($"stack-volume-replace-platform-{suffix}");
            await uow.Platforms.AddAsync(platform, cancellationToken);

            var stack = Stack.Create(
                $"stack-volume-replace-{suffix}",
                actorId,
                StackSource.WebEditor,
                platform.Id,
                new ManualStack(
                    """
                    services:
                      db:
                        image: postgres
                        volumes:
                          - db-data:/var/lib/postgresql/data
                    volumes:
                      db-data:
                    """,
                    StackUpdateBehavior.Disabled));
            await uow.Stacks.AddAsync(stack, cancellationToken);
            await uow.Stacks.ReplaceReleaseVolumeBindingsAsync(
                stack.CurrentStackReleaseId,
                [
                    new StackReleaseVolumeBinding(
                        stack.CurrentStackReleaseId,
                        platform.Id,
                        "db-data",
                        "db-data",
                        isExternal: false,
                        isAnonymous: false)
                ],
                cancellationToken);

            await uow.CommitAsync(cancellationToken);
            releaseId = stack.CurrentStackReleaseId;
            platformId = platform.Id;
        }

        await using (var scope = Services.CreateAsyncScope())
        {
            var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            await uow.Stacks.ReplaceReleaseVolumeBindingsAsync(
                releaseId,
                [
                    new StackReleaseVolumeBinding(
                        releaseId,
                        platformId,
                        "db-data",
                        "db-data",
                        isExternal: true,
                        isAnonymous: false)
                ],
                cancellationToken);
            await uow.CommitAsync(cancellationToken);

            var storedBindings = await uow.Stacks.GetReleaseVolumeBindingsAsync(releaseId, cancellationToken);
            var binding = Assert.Single(storedBindings);
            Assert.Equal("db-data", binding.VolumeName);
            Assert.True(binding.IsExternal);
        }
    }

    private static Platform CreatePlatform(string name)
        => new(
            name,
            "unix:///var/run/docker.sock",
            networkCount: 0,
            volumeCount: 0,
            imageCount: 0,
            cpuCount: 1,
            memTotal: 1024,
            serverVersion: "test",
            agentVersion: null,
            PlatformStatus.Online,
            PlatformConnectorType.Local,
            new DockerPlatformDescriptor("daemon", 0, 0, 0, 0));
}
