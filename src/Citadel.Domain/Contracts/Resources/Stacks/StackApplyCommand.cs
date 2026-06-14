using Domain.Entities.Stacks;

namespace Domain.Contracts.Resources.Stacks;

public sealed record StackApplyCommand(
    string PlatformAddress,
    string StackName,
    string ComposeFileContent,
    string? ProjectName,
    string? EnvironmentFilePath,
    string? RegistryAuth,
    string? RegistryName,
    IReadOnlyList<string>? EnvironmentVariables,
    StackCommand? PreDeploy,
    StackCommand? PostDeploy,
    StackSpec Spec);
