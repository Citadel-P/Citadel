using Domain;
using Domain.Contracts.Resources.Stacks;
using Domain.Entities.Stacks;
using AgentStackApplyEventType = Citadel.Stacks.V1.StackApplyEventType;
using AgentStackApplyRequest = Citadel.Stacks.V1.StackApplyRequest;
using AgentStackApplyResponse = Citadel.Stacks.V1.StackApplyResponse;
using AgentStackCommand = Citadel.Stacks.V1.StackCommand;
using AgentStackSecretFile = Citadel.Stacks.V1.StackSecretFile;

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
            SecretTargetServiceNames: cmd.SecretTargetServiceNames);
    }

    public static StackApplyResult Map(this Hosting.DockerClient.Models.Stacks.StackApplyResult result)
    {
        return new StackApplyResult(
            Type: result.Type.Map(),
            Message: result.Message,
            ExitCode: result.ExitCode);
    }

    public static AgentStackApplyRequest ToAgentRequest(this StackApplyCommand cmd)
    {
        var request = new AgentStackApplyRequest
        {
            StackName = cmd.StackName,
            DestroyBeforeDeploy = cmd.DestroyBeforeDeploy,
            PullImages = cmd.PullImages
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

        return request;
    }

    public static StackApplyResult Map(this AgentStackApplyResponse result)
    {
        return new StackApplyResult(
            Type: result.Type.Map(),
            Message: result.HasMessage ? result.Message : null,
            ExitCode: result.HasExitCode ? result.ExitCode : null);
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
