using Domain;
using Domain.Contracts.Resources.Stacks;
using Domain.Entities.Stacks;

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
            PullImages: cmd.PullImages);
    }

    public static StackApplyResult Map(this Hosting.DockerClient.Models.Stacks.StackApplyResult result)
    {
        return new StackApplyResult(
            Type: result.Type.Map(),
            Message: result.Message,
            ExitCode: result.ExitCode);
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

    private static Hosting.DockerClient.Models.Stacks.StackCommand? Map(this StackCommand cmd)
    {
        if (cmd is null)
            return null;
        return new Hosting.DockerClient.Models.Stacks.StackCommand
        (
            Commands: cmd.Commands,
            Path: cmd.Path
        );
    }

}
