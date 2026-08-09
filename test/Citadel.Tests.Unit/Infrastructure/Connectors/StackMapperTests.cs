using Domain;
using Domain.Contracts.Resources.Stacks;
using Domain.Entities.Stacks;
using Google.Protobuf;
using Infrastructure.Connectors.Mappers;

namespace Tests.Unit.Infrastructure.Connectors;

public sealed class StackMapperTests
{
    [Fact]
    public void SwarmOrchestrationMode_ShouldBeEquivalentForLocalAgentAndEdgePayloads()
    {
        var command = new StackApplyCommand(
            PlatformAddress: "unix:///var/run/docker.sock",
            StackName: "demo",
            ComposeFileContent: "services:\n  api:\n    image: nginx\n",
            ProjectName: "demo",
            EnvironmentFilePath: null,
            RegistryAuth: null,
            RegistryName: null,
            RegistryHost: null,
            DestroyBeforeDeploy: false,
            EnvironmentVariables: null,
            PreDeploy: null,
            PostDeploy: null,
            Spec: new ManualStack(
                "services:\n  api:\n    image: nginx\n",
                StackUpdateBehavior.Disabled,
                DestroyBeforeDeploy: false),
            OrchestrationMode: StackOrchestrationMode.DockerSwarm,
            SourceFiles: [new StackSourceFile("source/compose.yml", [1, 2, 3])],
            RetainedSwarmSecrets:
            [
                new StackRetainedSwarmSecret(
                    "citadel-api-v1",
                    "demo_citadel-api-v1",
                    [new StackReleaseSwarmResourceMount("api", "api-key")])
            ],
            RetainedSwarmConfigs:
            [
                new StackRetainedSwarmConfig(
                    "settings-v1",
                    "demo_settings-v1",
                    [new StackReleaseSwarmResourceMount("api", "/etc/demo/settings.yml")])
            ],
            ConvertComposeProjectToSwarm: true);

        var local = command.ToCommand();
        var transport = command.ToAgentRequest();

        Assert.Equal(
            Hosting.DockerClient.Models.Stacks.StackOrchestrationMode.DockerSwarm,
            local.OrchestrationMode);
        Assert.Equal(Citadel.Stacks.V1.StackOrchestrationMode.DockerSwarm, transport.OrchestrationMode);
        Assert.True(local.ConvertComposeProjectToSwarm);
        Assert.True(transport.ConvertComposeProjectToSwarm);
        Assert.Equal([1, 2, 3], Assert.Single(local.SourceFiles!).Content);
        Assert.Equal("source/compose.yml", Assert.Single(transport.SourceFiles).RelativePath);
        Assert.Equal("demo_citadel-api-v1", Assert.Single(local.RetainedSwarmSecrets!).DockerResourceName);
        Assert.Equal("api-key", Assert.Single(Assert.Single(transport.RetainedSwarmSecrets).Mounts).TargetName);
        Assert.Equal("demo_settings-v1", Assert.Single(local.RetainedSwarmConfigs!).DockerResourceName);
        Assert.Equal("/etc/demo/settings.yml", Assert.Single(Assert.Single(transport.RetainedSwarmConfigs).Mounts).TargetName);
        Assert.Equal(
            Citadel.Stacks.V1.StackOrchestrationMode.DockerSwarm,
            Citadel.Stacks.V1.StackApplyRequest.Parser.ParseFrom(transport.ToByteArray()).OrchestrationMode);
        Assert.True(Citadel.Stacks.V1.StackApplyRequest.Parser.ParseFrom(transport.ToByteArray()).ConvertComposeProjectToSwarm);
        Assert.Equal(
            [1, 2, 3],
            Assert.Single(Citadel.Stacks.V1.StackApplyRequest.Parser.ParseFrom(transport.ToByteArray()).SourceFiles).Content.ToByteArray());
        Assert.Equal(
            "demo_citadel-api-v1",
            Assert.Single(Citadel.Stacks.V1.StackApplyRequest.Parser.ParseFrom(transport.ToByteArray()).RetainedSwarmSecrets).DockerResourceName);
        Assert.Equal(
            "demo_settings-v1",
            Assert.Single(Citadel.Stacks.V1.StackApplyRequest.Parser.ParseFrom(transport.ToByteArray()).RetainedSwarmConfigs).DockerResourceName);
    }
}
