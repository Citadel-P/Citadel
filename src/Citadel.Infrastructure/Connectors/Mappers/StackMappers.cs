using Domain;
using Domain.Contracts.Resources.Stacks;
using Domain.Entities.Stacks;
using AgentStackApplyEventType = Citadel.Stacks.V1.StackApplyEventType;
using AgentStackApplyRequest = Citadel.Stacks.V1.StackApplyRequest;
using AgentStackApplyResponse = Citadel.Stacks.V1.StackApplyResponse;
using AgentStackCommand = Citadel.Stacks.V1.StackCommand;
using AgentStackSecretFile = Citadel.Stacks.V1.StackSecretFile;
using AgentStackSourceFile = Citadel.Stacks.V1.StackSourceFile;
using AgentStackRetainedSwarmSecret = Citadel.Stacks.V1.StackRetainedSwarmSecret;
using AgentStackRetainedSwarmSecretMount = Citadel.Stacks.V1.StackRetainedSwarmSecretMount;
using AgentStackRetainedSwarmConfig = Citadel.Stacks.V1.StackRetainedSwarmConfig;
using AgentStackRetainedSwarmConfigMount = Citadel.Stacks.V1.StackRetainedSwarmConfigMount;

namespace Infrastructure.Connectors.Mappers;

internal static class StackMappers
{
    public static Hosting.DockerClient.Models.Stacks.StackApplyCommand ToCommand(this StackApplyCommand cmd)
    {
        return new Hosting.DockerClient.Models.Stacks.StackApplyCommand(
            PlatformAddress: cmd.PlatformAddress,
            StackName: cmd.StackName,
            ComposeFileContent: cmd.ComposeFileContent,
            ProjectName: cmd.ProjectName,
            EnvironmentFilePath: cmd.EnvironmentFilePath,
            EnvironmentVariables: cmd.EnvironmentVariables,
            DestroyBeforeDeploy: cmd.DestroyBeforeDeploy,
            RegistryAuth: cmd.RegistryAuth,
            RegistryName: cmd.RegistryName,
            RegistryHost: cmd.RegistryHost,
            PreDeploy: cmd.PreDeploy?.Map(),
            PostDeploy: cmd.PostDeploy?.Map(),
            ServiceNames: cmd.ServiceNames,
            PullImages: cmd.PullImages,
            SourceWorkingDirectory: cmd.SourceWorkingDirectory,
            SourceComposeFilePaths: cmd.SourceComposeFilePaths,
            SourceEnvFilePaths: cmd.SourceEnvFilePaths,
            LabelsOverrideFilePath: cmd.LabelsOverrideFilePath,
            GeneratedFilesDirectory: cmd.GeneratedFilesDirectory,
            SecretFiles: cmd.SecretFiles?.Select(secret => new Hosting.DockerClient.Models.Stacks.StackSecretFile(
                secret.Name,
                secret.TargetPath,
                secret.Content)).ToArray(),
            SecretTargetServiceNames: cmd.SecretTargetServiceNames,
            OrchestrationMode: cmd.OrchestrationMode == StackOrchestrationMode.DockerSwarm
                ? Hosting.DockerClient.Models.Stacks.StackOrchestrationMode.DockerSwarm
                : Hosting.DockerClient.Models.Stacks.StackOrchestrationMode.DockerCompose,
            SourceFiles: cmd.SourceFiles?.Select(file => new Hosting.DockerClient.Models.Stacks.StackSourceFile(
                file.RelativePath,
                file.Content)).ToArray(),
            RetainedSwarmSecrets: cmd.RetainedSwarmSecrets?.Select(secret =>
                new Hosting.DockerClient.Models.Stacks.StackRetainedSwarmSecret(
                    secret.ComposeResourceName,
                    secret.DockerResourceName,
                    secret.Mounts.Select(static mount =>
                        new Hosting.DockerClient.Models.Stacks.StackRetainedSwarmSecretMount(
                            mount.ServiceName,
                            mount.TargetName)).ToArray())).ToArray(),
            RetainedSwarmConfigs: cmd.RetainedSwarmConfigs?.Select(config =>
                new Hosting.DockerClient.Models.Stacks.StackRetainedSwarmConfig(
                    config.ComposeResourceName,
                    config.DockerResourceName,
                    config.Mounts.Select(static mount =>
                        new Hosting.DockerClient.Models.Stacks.StackRetainedSwarmConfigMount(
                            mount.ServiceName,
                            mount.TargetName)).ToArray())).ToArray(),
            ConvertComposeProjectToSwarm: cmd.ConvertComposeProjectToSwarm);
    }

    public static StackApplyResult Map(this Hosting.DockerClient.Models.Stacks.StackApplyResult result)
    {
        return new StackApplyResult(
            Type: result.Type.Map(),
            Message: result.Message,
            ExitCode: result.ExitCode,
            StackStatus: ParseStatus(result.StackStatus));
    }

    public static AgentStackApplyRequest ToAgentRequest(this StackApplyCommand cmd)
    {
        var request = new AgentStackApplyRequest
        {
            StackName = cmd.StackName,
            DestroyBeforeDeploy = cmd.DestroyBeforeDeploy,
            PullImages = cmd.PullImages,
            OrchestrationMode = cmd.OrchestrationMode == StackOrchestrationMode.DockerSwarm
                ? Citadel.Stacks.V1.StackOrchestrationMode.DockerSwarm
                : Citadel.Stacks.V1.StackOrchestrationMode.DockerCompose,
            ConvertComposeProjectToSwarm = cmd.ConvertComposeProjectToSwarm
        };

        if (cmd.ComposeFileContent is not null)
            request.ComposeFileContent = cmd.ComposeFileContent;
        if (cmd.ProjectName is not null)
            request.ProjectName = cmd.ProjectName;
        if (cmd.EnvironmentFilePath is not null)
            request.EnvironmentFilePath = cmd.EnvironmentFilePath;
        if (cmd.RegistryAuth is not null)
            request.RegistryAuth = cmd.RegistryAuth;
        if (cmd.RegistryName is not null)
            request.RegistryName = cmd.RegistryName;
        if (cmd.RegistryHost is not null)
            request.RegistryHost = cmd.RegistryHost;
        if (cmd.PreDeploy is not null)
            request.PreDeploy = cmd.PreDeploy.MapAgent();
        if (cmd.PostDeploy is not null)
            request.PostDeploy = cmd.PostDeploy.MapAgent();
        if (cmd.SourceWorkingDirectory is not null)
            request.SourceWorkingDirectory = cmd.SourceWorkingDirectory;
        if (cmd.LabelsOverrideFilePath is not null)
            request.LabelsOverrideFilePath = cmd.LabelsOverrideFilePath;
        if (cmd.GeneratedFilesDirectory is not null)
            request.GeneratedFilesDirectory = cmd.GeneratedFilesDirectory;

        request.EnvironmentVariables.AddRange(cmd.EnvironmentVariables ?? []);
        request.ServiceNames.AddRange(cmd.ServiceNames ?? []);
        request.SourceComposeFilePaths.AddRange(cmd.SourceComposeFilePaths ?? []);
        request.SourceEnvFilePaths.AddRange(cmd.SourceEnvFilePaths ?? []);
        request.SecretTargetServiceNames.AddRange(cmd.SecretTargetServiceNames ?? []);

        if (cmd.SecretFiles is not null)
        {
            request.SecretFiles.AddRange(cmd.SecretFiles.Select(secret => new AgentStackSecretFile
            {
                Name = secret.Name,
                TargetPath = secret.TargetPath,
                Content = secret.Content
            }));
        }

        if (cmd.SourceFiles is not null)
        {
            request.SourceFiles.AddRange(cmd.SourceFiles.Select(file => new AgentStackSourceFile
            {
                RelativePath = file.RelativePath,
                Content = Google.Protobuf.ByteString.CopyFrom(file.Content)
            }));
        }

        if (cmd.RetainedSwarmSecrets is not null)
        {
            request.RetainedSwarmSecrets.AddRange(cmd.RetainedSwarmSecrets.Select(secret =>
            {
                var mapped = new AgentStackRetainedSwarmSecret
                {
                    ComposeResourceName = secret.ComposeResourceName,
                    DockerResourceName = secret.DockerResourceName
                };
                mapped.Mounts.AddRange(secret.Mounts.Select(static mount => new AgentStackRetainedSwarmSecretMount
                {
                    ServiceName = mount.ServiceName,
                    TargetName = mount.TargetName
                }));
                return mapped;
            }));
        }

        if (cmd.RetainedSwarmConfigs is not null)
        {
            request.RetainedSwarmConfigs.AddRange(cmd.RetainedSwarmConfigs.Select(config =>
            {
                var mapped = new AgentStackRetainedSwarmConfig
                {
                    ComposeResourceName = config.ComposeResourceName,
                    DockerResourceName = config.DockerResourceName
                };
                mapped.Mounts.AddRange(config.Mounts.Select(static mount => new AgentStackRetainedSwarmConfigMount
                {
                    ServiceName = mount.ServiceName,
                    TargetName = mount.TargetName
                }));
                return mapped;
            }));
        }

        return request;
    }

    public static StackApplyResult Map(this AgentStackApplyResponse result)
    {
        return new StackApplyResult(
            Type: result.Type.Map(),
            Message: result.HasMessage ? result.Message : null,
            ExitCode: result.HasExitCode ? result.ExitCode : null,
            StackStatus: result.HasStackStatus ? ParseStatus(result.StackStatus) : null);
    }

    private static StackApplyEventType Map(this Hosting.DockerClient.Models.Stacks.StackApplyEventType type)
    {
        return type switch
        {
            Hosting.DockerClient.Models.Stacks.StackApplyEventType.StdOut => StackApplyEventType.StdOut,
            Hosting.DockerClient.Models.Stacks.StackApplyEventType.StdErr => StackApplyEventType.StdErr,
            Hosting.DockerClient.Models.Stacks.StackApplyEventType.SystemMessage => StackApplyEventType.SystemMessage,
            Hosting.DockerClient.Models.Stacks.StackApplyEventType.CommandCompleted => StackApplyEventType.CommandCompleted,

            _ => throw new ArgumentOutOfRangeException(nameof(type), type, null)
        };
    }

    private static StackApplyEventType Map(this AgentStackApplyEventType type)
    {
        return type switch
        {
            AgentStackApplyEventType.StdOut => StackApplyEventType.StdOut,
            AgentStackApplyEventType.StdErr => StackApplyEventType.StdErr,
            AgentStackApplyEventType.SystemMessage => StackApplyEventType.SystemMessage,
            AgentStackApplyEventType.CommandCompleted => StackApplyEventType.CommandCompleted,

            _ => throw new ArgumentOutOfRangeException(nameof(type), type, null)
        };
    }

    private static StackReleaseStatus? ParseStatus(string? value)
        => Enum.TryParse<StackReleaseStatus>(value, ignoreCase: true, out var status)
            ? status
            : null;

    private static Hosting.DockerClient.Models.Stacks.StackCommand? Map(this StackCommand cmd)
    {
        if (cmd is null)
            return null;

        return new Hosting.DockerClient.Models.Stacks.StackCommand(
            Commands: cmd.Commands,
            Path: cmd.Path);
    }

    private static AgentStackCommand MapAgent(this StackCommand cmd)
    {
        var command = new AgentStackCommand
        {
            Path = cmd.Path
        };
        command.Commands.AddRange(cmd.Commands ?? []);
        return command;
    }
}
