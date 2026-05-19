using Domain.Contracts.Interfaces;
using Domain.Entities.Identity;
using Hosting.Common;
using Microsoft.Extensions.DependencyInjection;
using Tests.Integration.Helpers;
using Infrastructure.Persistence;

namespace Tests.Integration.Application.Features.Permissions;

public class HasPermissionForAllIntegrationTests(PostgresTestFixture fixture) : IntegrationTestBase(fixture)
{
    protected override async ValueTask SeedDbAsync(IUnitOfWork uow)
    {
        // Create two deployments
        var platform = Fakes.GetDummyPlatform();
        await uow.Platforms.AddAsync(platform, TestContext.Current.CancellationToken);

        var depA = new Domain.Entities.Deployments.Deployment(
            name: "dep-a",
            createdByActorId: Constants.SystemId,
            platformId: platform.Id,
            spec: new Domain.Entities.Deployments.DeploymentSpec(new Domain.Entities.Deployments.ExternalImage(Guid.Empty, "img:a"), Domain.UpdateBehavior.Notify));
        var depB = new Domain.Entities.Deployments.Deployment(
            name: "dep-b",
            createdByActorId: Constants.SystemId,
            platformId: platform.Id,
            spec: new Domain.Entities.Deployments.DeploymentSpec(new Domain.Entities.Deployments.ExternalImage(Guid.Empty, "img:b"), Domain.UpdateBehavior.Notify));

        await uow.Deployments.AddAsync(depA, TestContext.Current.CancellationToken);
        await uow.Deployments.AddAsync(depB, TestContext.Current.CancellationToken);

        await uow.CommitAsync(TestContext.Current.CancellationToken);
    }

    [Fact]
    public async Task HasPermissionForAll_Should_Return_False_When_Not_All_Resources_Are_Granted()
    {
        var depA = await GetDeploymentIdByNameAsync("dep-a");
        var depB = await GetDeploymentIdByNameAsync("dep-b");

        // create subject and grant only first deployment
        var subject = await CreateAuthorizationSubjectAsync(resourceGrants: new[] { new ResourceGrant(ResourceType.Deployment, depA, PermissionLevel.Read) });

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var actorIds = await uow.Users.GetActorScopeAsync(subject.UserId, TestContext.Current.CancellationToken);

        var resA = await uow.Users.GetEffectivePermissionsAsync(actorIds, ResourceType.Deployment, depA, TestContext.Current.CancellationToken);
        var resB = await uow.Users.GetEffectivePermissionsAsync(actorIds, ResourceType.Deployment, depB, TestContext.Current.CancellationToken);

        Assert.False(resA.Has(PermissionLevel.Read, SpecificPermission.None) && resB.Has(PermissionLevel.Read, SpecificPermission.None));
    }

    [Fact]
    public async Task HasPermissionForAll_Should_Return_True_When_All_Resources_Are_Granted()
    {
        var depA = await GetDeploymentIdByNameAsync("dep-a");
        var depB = await GetDeploymentIdByNameAsync("dep-b");

        var subject = await CreateAuthorizationSubjectAsync(resourceGrants: new[] { new ResourceGrant(ResourceType.Deployment, depA, PermissionLevel.Read) });

        // grant second deployment via repository
        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        // use the factory method to create ResourceAccess
        var access = ResourceAccess.Create(ResourceType.Deployment, depB, subject.ActorId, PermissionLevel.Read, null);
        await uow.ResourceAccesses.AddAsync(access, TestContext.Current.CancellationToken);
        await uow.CommitAsync(TestContext.Current.CancellationToken);

        var actorIds = await uow.Users.GetActorScopeAsync(subject.UserId, TestContext.Current.CancellationToken);
        var resA = await uow.Users.GetEffectivePermissionsAsync(actorIds, ResourceType.Deployment, depA, TestContext.Current.CancellationToken);
        var resB = await uow.Users.GetEffectivePermissionsAsync(actorIds, ResourceType.Deployment, depB, TestContext.Current.CancellationToken);

        Assert.True(resA.Has(PermissionLevel.Read, SpecificPermission.None) && resB.Has(PermissionLevel.Read, SpecificPermission.None));
    }

    private async Task<Guid> GetDeploymentIdByNameAsync(string name)
    {
        await using var scope = Services.CreateAsyncScope();
        var connectionFactory = scope.ServiceProvider.GetRequiredService<IDbConnectionFactory>();
        await using var connection = connectionFactory.Create();
        await connection.OpenAsync(TestContext.Current.CancellationToken);
        await using var tx = await connection.BeginTransactionAsync(TestContext.Current.CancellationToken);

        await using var cmd = connection.CreateCommand();
        cmd.Transaction = tx;
        cmd.CommandText = "SELECT Id FROM Deployments WHERE Name = @Name LIMIT 1;";
        var p = cmd.CreateParameter();
        p.ParameterName = "@Name";
        p.Value = name;
        cmd.Parameters.Add(p);

        var id = await cmd.ExecuteScalarAsync(TestContext.Current.CancellationToken);
        await tx.CommitAsync(TestContext.Current.CancellationToken);

        return Guid.Parse(id.ToString()!);
    }
}
