using Dapper;
using System.Net.Http.Json;
using Application.TaskJobs;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Activities;
using Domain.Entities.Platforms;
using Hosting.Common;
using Microsoft.Extensions.DependencyInjection;
using Moq;
using Npgsql;
using System.Text.Json;

namespace Tests.Integration.Application.Features.Platforms;

public class PlatformDeleteTests(PostgresTestFixture fixture) : IntegrationTestBase(fixture)
{
    private readonly Mock<IPlatformHealthMonitorJob> healthMonitorMock = new();
    private Guid platformId;
    private Guid backupRunId;

    protected override void ConfigureTestServices(IServiceCollection services)
    {
        services.AddSingleton(_ => healthMonitorMock.Object);
    }

    protected override async ValueTask SeedDbAsync(IUnitOfWork _)
    {
        platformId = Guid.CreateVersion7();
        backupRunId = Guid.CreateVersion7();
        var secretId = Guid.CreateVersion7();
        var repositoryId = Guid.CreateVersion7();
        var policyId = Guid.CreateVersion7();
        var platformDescriptor = JsonSerializer.Serialize(
            new DockerPlatformDescriptor(
                DaemonId: "123456",
                ContainerCount: 5,
                ContainersRunning: 2,
                ContainersPaused: 2,
                ContainersStopped: 1),
            PlatformJsonContext.Default.PlatformDescriptor);

        await using var connection = Services.GetRequiredService<NpgsqlDataSource>().CreateConnection();
        await connection.ExecuteAsync(
            """
            INSERT INTO platforms (
                id, name, address, description, networkcount, volumecount, imagecount, cpucount,
                memtotal, serverversion, agentversion, status, connectortype, platformdescriptor)
            VALUES (
                @PlatformId, 'P-DELETE', 'https://delete.address', NULL, 1, 2, 3, 4,
                500, '1.0.0', '1.0.0', 'Online', 'Agent', @PlatformDescriptor::json);

            INSERT INTO secretdefinitions (id, name, providertype)
            VALUES (@SecretId, 'PLATFORM_DELETE_BACKUP_PASSWORD', 'InternalEncrypted');

            INSERT INTO internalsecretvalues (secretid, encryptedvalue)
            VALUES (@SecretId, 'encrypted-value');

            INSERT INTO backuprepositories (
                id, name, normalizedname, description, type, spec, passwordsecretid, status,
                controlstate, createdbyactorid)
            VALUES (
                @RepositoryId, 'platform-delete-repository', 'platform-delete-repository', NULL, 'FileSystem',
                '{"$type":"FileSystem","location":"Core","path":"/tmp/platform-delete-backups"}'::jsonb,
                @SecretId, 'Unknown', 'Idle', @ActorId);

            INSERT INTO backuppolicies (
                id, name, normalizedname, source, backuprepositoryid, enabled, keeplastsuccessful,
                timeoutseconds, alertonfailure, runasactorid, createdbyactorid)
            VALUES (
                @PolicyId, 'platform-delete-policy', 'platform-delete-policy',
                @Source::jsonb, @RepositoryId, TRUE, 14, 14400, FALSE, @ActorId, @ActorId);

            INSERT INTO backupruns (
                id, backuppolicyid, backuprepositoryid, policynamesnapshot, sourcesnapshot,
                repositorytypesnapshot, trigger, status, snapshotavailability, resticsnapshotid,
                filesprocessed, bytesprocessed, bytesadded, completedat, triggeredbyactorid)
            VALUES (
                @BackupRunId, @PolicyId, @RepositoryId, 'platform-delete-policy', @Source::jsonb,
                'FileSystem', 'Manual', 'Succeeded', 'Available', 'snapshot-platform-delete',
                1, 128, 64, @UtcNow, @ActorId);

            INSERT INTO backuprunitems (
                id, backuprunid, platformid, volumename, status, resticsnapshotid,
                filesprocessed, bytesprocessed, bytesadded, startedat, completedat)
            VALUES (
                @BackupRunItemId, @BackupRunId, @PlatformId, 'platform-delete-volume',
                'Succeeded', 'snapshot-platform-delete', 1, 128, 64, @UtcNow, @UtcNow);
            """,
            new
            {
                PlatformId = platformId,
                PlatformDescriptor = platformDescriptor,
                SecretId = secretId,
                RepositoryId = repositoryId,
                PolicyId = policyId,
                BackupRunId = backupRunId,
                BackupRunItemId = Guid.CreateVersion7(),
                ActorId = Constants.SystemId,
                UtcNow = DateTime.UtcNow,
                Source = $$"""{"$type":"DockerVolume","platformId":"{{platformId}}","volumeName":"platform-delete-volume"}"""
            });
    }

    [Fact]
    public async Task Delete_Platform_Should_Delete_Entity_And_Add_Activity()
    {
        // Arrange
        healthMonitorMock
            .Setup(x => x.UntrackPlatform("https://delete.address", It.IsAny<CancellationToken>()))
            .ReturnsAsync(true);

        var request = new HttpRequestMessage(HttpMethod.Delete, "/api/v1/platforms")
        {
            Content = JsonContent.Create(new { ids = new[] { platformId } })
        };

        // Act
        var response = await Client.SendAsync(request, TestContext.Current.CancellationToken);

        // Assert
        response.EnsureSuccessStatusCode();

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var exists = await uow.Platforms.ExistsAsync(platformId, TestContext.Current.CancellationToken);
        var backupRunItems = await uow.BackupRunItems.GetByRunAsync(backupRunId, TestContext.Current.CancellationToken);
        var activities = await uow.ActivityEventRepository.GetPagedAsync(
            platformId,
            ActivityResourceType.Platform,
            ActivityEventType.PlatformDeleted,
            1,
            10,
            TestContext.Current.CancellationToken);

        var activitySummary = Assert.Single(activities.Items);
        var activity = await uow.ActivityEventRepository.GetByIdAsync(activitySummary.Id, TestContext.Current.CancellationToken);
        var deleted = Assert.IsType<PlatformDeleted>(activity?.Info);

        Assert.False(exists);
        Assert.Empty(backupRunItems);
        Assert.Equal(platformId, deleted.Platform.Id);
        Assert.Equal("P-DELETE", deleted.Platform.Name);
        Assert.Equal("https://delete.address", deleted.Platform.Address);
        healthMonitorMock.Verify(x => x.UntrackPlatform("https://delete.address", It.IsAny<CancellationToken>()), Times.Once);
    }
}
