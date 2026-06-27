using Domain.Entities.Stacks;

namespace Domain.Contracts.Resources.Stacks;

public sealed record StackApplyCommand(
    string PlatformAddress,
    string StackName,
    string? ComposeFileContent,
    string? ProjectName,
    string? EnvironmentFilePath,
    string? RegistryAuth,
    string? RegistryName,
    string? RegistryHost,
    bool DestroyBeforeDeploy,
    IReadOnlyList<string>? EnvironmentVariables,
    StackCommand? PreDeploy,
    StackCommand? PostDeploy,
    StackSpec Spec,
    IReadOnlyList<string>? ServiceNames = null,
    bool PullImages = false,
    string? SourceWorkingDirectory = null,
    IReadOnlyList<string>? SourceComposeFilePaths = null,
    IReadOnlyList<string>? SourceEnvFilePaths = null,
    string? LabelsOverrideFilePath = null,
    string? GeneratedFilesDirectory = null);
