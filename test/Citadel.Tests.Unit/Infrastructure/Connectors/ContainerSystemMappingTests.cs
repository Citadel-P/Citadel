using Citadel.SharedModels.V1;
using Domain;
using Infrastructure.Connectors.Mappers;

namespace Tests.Unit.Infrastructure.Connectors;

public class ContainerSystemMappingTests
{
    [Theory]
    [InlineData(true, "core", true, ContainerSystemRole.Core)]
    [InlineData(true, "DATABASE", true, ContainerSystemRole.Database)]
    [InlineData(true, "agent", true, ContainerSystemRole.Agent)]
    [InlineData(true, "EDGE-AGENT", true, ContainerSystemRole.EdgeAgent)]
    [InlineData(true, "future-role", true, null)]
    [InlineData(false, "core", false, null)]
    public void ContainerMessage_Map_PreservesProtectionAndParsesKnownRoles(
        bool isSystem,
        string role,
        bool expectedSystem,
        ContainerSystemRole? expectedRole)
    {
        var message = new ContainerMessage
        {
            Id = "container-id",
            Name = "/container",
            Image = "image:latest",
            ImageID = "sha256:image",
            IsSystem = isSystem,
            SystemRole = role
        };

        var result = message.Map();

        Assert.Equal(expectedSystem, result.IsSystem);
        Assert.Equal(expectedRole, result.SystemRole);
    }
}
