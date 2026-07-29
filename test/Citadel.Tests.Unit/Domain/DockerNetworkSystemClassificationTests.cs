using Domain.Contracts.Resources.Networks;

namespace Tests.Unit.Domain;

public sealed class DockerNetworkSystemClassificationTests
{
    [Theory]
    [InlineData("bridge", false, true)]
    [InlineData("HOST", false, true)]
    [InlineData("none", false, true)]
    [InlineData("nat", false, true)]
    [InlineData("docker_gwbridge", false, true)]
    [InlineData("swarm-ingress", true, true)]
    [InlineData("application-network", false, false)]
    [InlineData("bridge-app", false, false)]
    public void IsSystem_ClassifiesDaemonOwnedNetworks(string name, bool ingress, bool expected)
    {
        var network = new DockerNetworkResult(
            Name: name,
            Id: "network-id",
            Created: "",
            Driver: "bridge",
            Scope: "local",
            EnableIPv4: true,
            EnableIPv6: false,
            Internal: false,
            Attachable: false,
            Ingress: ingress,
            ConfigOnly: false,
            InUse: false,
            ConfigFrom: null,
            Ipam: null,
            Options: new Dictionary<string, string>(),
            Labels: new Dictionary<string, string>());

        Assert.Equal(expected, network.IsSystem);
    }
}
