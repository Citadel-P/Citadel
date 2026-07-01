namespace Domain.Contracts.Resources.Stacks;

public sealed record StackSecretFile(
    string Name,
    string TargetPath,
    string Content);
