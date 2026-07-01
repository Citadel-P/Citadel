using Domain.Entities.Identity;
using Hosting.Common;
using Hosting.Common.Attributes;

namespace Tests.Unit.Domain.Entities.Identity;

public sealed class PermissionTests
{
    [Fact]
    public void Admin_Permissions_Should_Include_All_Current_Specific_Capabilities()
    {
        Assert.True(Helpers.AdminPermissions.Has(PermissionLevel.Read, SpecificPermission.ResourceBindings));
        Assert.True(Helpers.AdminPermissions.Has(PermissionLevel.Read, SpecificPermission.Releases));
    }

    [Fact]
    public void Deployment_Execute_Permission_Should_Include_ResourceBindings()
    {
        var specifics = PermissionMatrix.GetAll()[ResourceType.Deployment]
            .SpecificPermissionMinimumLevels
            .Keys;

        var mask = (SpecificPermission)Permission.ToSpecificPermissionsMask(specifics);

        Assert.True((mask & SpecificPermission.ResourceBindings) == SpecificPermission.ResourceBindings);
    }

    [Fact]
    public void Stack_Execute_Permission_Should_Include_ResourceBindings_And_Releases()
    {
        var specifics = PermissionMatrix.GetAll()[ResourceType.Stack]
            .SpecificPermissionMinimumLevels
            .Keys;

        var mask = (SpecificPermission)Permission.ToSpecificPermissionsMask(specifics);

        Assert.True((mask & SpecificPermission.ResourceBindings) == SpecificPermission.ResourceBindings);
        Assert.True((mask & SpecificPermission.Releases) == SpecificPermission.Releases);
    }
}
