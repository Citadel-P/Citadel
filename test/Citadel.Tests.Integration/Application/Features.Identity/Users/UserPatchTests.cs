using System.Text;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Identity;
using Hosting.Common;
using Microsoft.Extensions.DependencyInjection;

namespace Tests.Integration.Application.Features.Identity.Users;

public class UserPatchTests : IntegrationTestBase
{
    [Fact]
    public async Task Patch_User_Should_Update_Email_Password_And_IsEnabled()
    {
        var seeded = await SeedUserAsync("user-patch", "user-patch@citadel.local");
        var patchJson = """
        {
          "email": "user-patched@citadel.local",
          "password": "newPassword123",
          "isEnabled": false
        }
        """;
        var content = new StringContent(patchJson, Encoding.UTF8, "application/merge-patch+json");

        var response = await Client.PatchAsync($"/api/v1/users/{seeded.UserId}", content, cancellationToken: TestContext.Current.CancellationToken);
        response.EnsureSuccessStatusCode();

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var user = await uow.Users.GetAsync(seeded.UserId, TestContext.Current.CancellationToken);
        var actor = await uow.Actors.GetById(seeded.ActorId, TestContext.Current.CancellationToken);

        Assert.NotNull(user);
        Assert.Equal("user-patched@citadel.local", user!.Email);
        Assert.True(User.IsValidPassword("newPassword123", user.Password));
        Assert.NotNull(actor);
        Assert.False(actor!.IsEnabled);
    }

    [Fact]
    public async Task Rename_User_Should_Update_Name()
    {
        var seeded = await SeedUserAsync("user-rename", "user-rename@citadel.local");
        var renameJson = $$"""
        {
          "id": "{{seeded.UserId}}",
          "name": "user-renamed"
        }
        """;
        var content = new StringContent(renameJson, Encoding.UTF8, "application/json");

        var response = await Client.PostAsync("/api/v1/users/rename", content, TestContext.Current.CancellationToken);
        response.EnsureSuccessStatusCode();

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var user = await uow.Users.GetAsync(seeded.UserId, TestContext.Current.CancellationToken);

        Assert.Equal("user-renamed", user?.Name);
    }

    [Fact]
    public async Task Add_User_Role_Should_Assign_Role()
    {
        var seeded = await SeedUserAsync("user-role-add", "user-role-add@citadel.local");
        var addRoleJson = $$"""
        {
          "roleId": "{{ViewerRoleId}}"
        }
        """;
        var content = new StringContent(addRoleJson, Encoding.UTF8, "application/json");

        var response = await Client.PostAsync($"/api/v1/users/{seeded.UserId}/roles", content, TestContext.Current.CancellationToken);
        response.EnsureSuccessStatusCode();

        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var roleIds = await uow.Users.GetActorRoleIdsAsync(seeded.ActorId, TestContext.Current.CancellationToken);

        Assert.Contains(ViewerRoleId, roleIds);
    }

    [Fact]
    public async Task Remove_User_Role_Should_Unassign_Role()
    {
        var seeded = await SeedUserAsync("user-role-remove", "user-role-remove@citadel.local");

        await using (var scope = Services.CreateAsyncScope())
        {
            var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            await uow.Users.AddActorRoleAsync(seeded.ActorId, ViewerRoleId, TestContext.Current.CancellationToken);
            await uow.CommitAsync(TestContext.Current.CancellationToken);
        }

        var response = await Client.DeleteAsync($"/api/v1/users/{seeded.UserId}/roles/{ViewerRoleId}", TestContext.Current.CancellationToken);
        response.EnsureSuccessStatusCode();

        await using var verificationScope = Services.CreateAsyncScope();
        var verificationUow = verificationScope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var roleIds = await verificationUow.Users.GetActorRoleIdsAsync(seeded.ActorId, TestContext.Current.CancellationToken);

        Assert.DoesNotContain(ViewerRoleId, roleIds);
    }

    private async Task<(Guid UserId, Guid ActorId)> SeedUserAsync(string name, string email, string password = "password123", bool isEnabled = true)
    {
        await using var scope = Services.CreateAsyncScope();
        var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var actor = Actor.Create(ActorType.User, new ActorMetadata(name), isEnabled);
        var user = new User(name, email, password, actor.Id, Constants.SystemId);

        await uow.Actors.AddAsync(actor, TestContext.Current.CancellationToken);
        await uow.Users.AddAsync(user, TestContext.Current.CancellationToken);
        await uow.CommitAsync(TestContext.Current.CancellationToken);

        return (user.Id, actor.Id);
    }
}
