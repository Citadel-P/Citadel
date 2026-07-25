using Application.Services.Licensing;
using Domain.Entities.Identity;
using Hosting.Common;

namespace Tests.Unit.Application.Services.Licensing;

public sealed class LicenseAccessControlPolicyTests
{
    private static readonly Guid RoleId = Guid.Parse("11111111-1111-1111-1111-111111111111");
    private static readonly Guid ActorId = Guid.Parse("22222222-2222-2222-2222-222222222222");
    private static readonly Guid ResourceId = Guid.Parse("33333333-3333-3333-3333-333333333333");

    [Theory]
    [InlineData(PermissionLevel.Execute, PermissionLevel.Write)]
    [InlineData(PermissionLevel.Execute, PermissionLevel.Read)]
    [InlineData(PermissionLevel.Write, PermissionLevel.Read)]
    public void ExpandsPermissions_Should_Not_Treat_A_Permission_Reduction_As_Expansion(
        PermissionLevel current,
        PermissionLevel proposed)
    {
        var existing = Permission.Create(RoleId, ResourceType.Stack, current);
        var candidate = Permission.Create(RoleId, ResourceType.Stack, proposed);

        var expands = LicenseAccessControlPolicy.ExpandsPermissions([existing], [candidate]);

        Assert.False(expands);
    }

    [Fact]
    public void ExpandsPermissions_Should_Detect_A_Higher_Permission_Level()
    {
        var existing = Permission.Create(RoleId, ResourceType.Stack, PermissionLevel.Read);
        var candidate = Permission.Create(RoleId, ResourceType.Stack, PermissionLevel.Write);

        var expands = LicenseAccessControlPolicy.ExpandsPermissions([existing], [candidate]);

        Assert.True(expands);
    }

    [Fact]
    public void ExpandsResourceAccess_Should_Use_The_Same_Hierarchical_Permission_Semantics()
    {
        var existing = ResourceAccess.Create(
            ResourceType.Stack,
            ResourceId,
            ActorId,
            PermissionLevel.Execute);
        var candidate = ResourceAccess.Create(
            ResourceType.Stack,
            ResourceId,
            ActorId,
            PermissionLevel.Read);

        var expands = LicenseAccessControlPolicy.ExpandsResourceAccess([existing], [candidate]);

        Assert.False(expands);
    }

    [Fact]
    public void ExpandsPermissions_Should_Detect_An_Added_Specific_Permission()
    {
        var existing = Permission.Create(
            RoleId,
            ResourceType.Stack,
            PermissionLevel.Read,
            [SpecificPermission.Logs]);
        var candidate = Permission.Create(
            RoleId,
            ResourceType.Stack,
            PermissionLevel.Read,
            [SpecificPermission.Logs, SpecificPermission.Inspect]);

        var expands = LicenseAccessControlPolicy.ExpandsPermissions([existing], [candidate]);

        Assert.True(expands);
    }
}
