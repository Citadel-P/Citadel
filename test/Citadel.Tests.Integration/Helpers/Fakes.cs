using Domain;
using Domain.Contracts.Resources.Containers;
using Domain.Contracts.Resources.Images;
using Domain.Contracts.Resources.Platforms;
using Domain.Entities.Platforms;

namespace Tests.Integration.Helpers;

internal static class Fakes
{
    internal static PlatformResult GetDummyPlatformResult() => new(
        Name: "p-01",
        Address: "https://original.address",
        NetworkCount: 1,
        VolumeCount: 2,
        ImageCount: 3,
        CpuCount: 4,
        MemTotal: 500,
        ServerVersion: "1.0.0",
        AgentVersion: "1.0.0",
        Descriptor: new DockerPlatformDescriptor(
            DaemonId: "123456",
            ContainerCount: 5,
            ContainersRunning: 2,
            ContainersPaused: 3,
            ContainersStopped: 0,
            Driver: "overlay2",
            OperatingSystem: "Linux",
            OsVersion: "5.15",
            OsType: "linux",
            Architecture: "x86_64"
        )
    );

    internal static Platform GetDummyPlatform() => new(
        name: "Docker-P-01",
        address: "https://original.address",
        networkCount: 1,
        volumeCount: 2,
        imageCount: 3,
        cpuCount: 4,
        memTotal: 500,
        serverVersion: "1.0.0",
        agentVersion: "1.0.0",
        status: PlatformStatus.Online,
        connectorType: PlatformConnectorType.Agent,
        platformDescriptor: new DockerPlatformDescriptor(
            DaemonId: "123456",
            ContainerCount: 5,
            ContainersRunning: 2,
            ContainersPaused: 2,
            ContainersStopped: 1
        )
    );

    internal static IEnumerable<DockerContainer> GetDummyContainers(int total = 3)
    {
        for (int i = 0; i < total; i++)
        {
            yield return new DockerContainer(
                Name: $"c-{i:D2}",
                Image: $"image-{i}:latest",
                ImageId: $"image-id-{i}:latest",
                State: ContainerStateStatus.Running,
                Id: $"{i}-docker-id"
            );
        }
    }

    internal static IReadOnlyList<ImageResult> GetDummyImages(int total = 3)
    {
        var result = new List<ImageResult>();
        for (int i = 0; i < total; i++)
        {
            result.Add(new ImageResult(
                Id: $"image-id-{i}:latest",
                RepoTags: [$"image-{i}:latest"],
                Size: 123456,
                Containers: i,
                Created: 9999999,
                SharedSize: 999,
                VirtualSize: 123456,
                ParentId: "p-012",
                RepoDigests: [],
                Labels: new Dictionary<string, string>()
            ));
        }

        return result;
    }
    
}
