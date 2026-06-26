using System.Diagnostics.CodeAnalysis;
using System.Text.Json.Serialization;
using Domain.Contracts.Resources;

namespace Domain.Entities.Stacks;

/// <param name="ProjectName">Optionally set a different compose project name. If importing existing stack, this should match the compose project name on your host.</param>
/// <param name="PreDeploy">A list of commands to run before deploying the stack. Useful for things like logging into a private registry, or running a script to generate config files.</param>
/// <param name="PostDeploy">A list of commands to run after deploying the stack. Useful for things like running database migrations, or seeding data.</param>
/// <param name="EnvVars"></param>
/// <param name="EnvFilePath">The path to write the file to, relative to the 'Run Directory'. </param>
/// <param name="RegistryId">The Id of the container registry to use for the stack.</param>
/// <param name="DestroyBeforeDeploy">Whether to destroy the stack before deploying it. This is useful for stacks that don't support rolling updates, or if you want to ensure a clean slate before deploying.</param>
[JsonPolymorphic]
[DynamicallyAccessedMembers(DynamicallyAccessedMemberTypes.All)]
[JsonDerivedType(typeof(ManualStack), nameof(StackSource.WebEditor))]
[JsonDerivedType(typeof(GitStack), nameof(StackSource.Git))]
public abstract record StackSpec(
    string? ProjectName,
    StackCommand? PreDeploy,
    StackCommand? PostDeploy,
    List<string>? EnvVars = null,
    string? EnvFilePath = null,
    Guid? RegistryId = null,
    bool DestroyBeforeDeploy = true
    );

public sealed record ManualStack(
    string ComposeFile,
    StackUpdateBehavior UpdateBehavior,
    List<string>? EnvVars = null,
    string? EnvFilePath = null,
    string? ProjectName = null,
    StackCommand? PreDeploy = null,
    StackCommand? PostDeploy = null,
    Guid? RegistryId = null,
    bool DestroyBeforeDeploy = true
) : StackSpec(ProjectName, PreDeploy, PostDeploy, EnvVars, EnvFilePath, RegistryId, DestroyBeforeDeploy);

/// <param name="GitRepoId">Reference to an existing Repo to attach to, <see cref="Git.GitRepository"/></param>
/// <param name="ProjectName"></param>
/// <param name="ComposePaths">Default to the root of the repository, but can specify subdirectory paths to look for compose files.</param>
/// <param name="PreDeploy"></param>
/// <param name="PostDeploy"></param>
/// <param name="EnvVars"></param>
/// <param name="AdditionalEnvFileFromRepo">Additional env files selected from the Repo.</param>
/// <param name="EnvFilePath"></param>
/// <param name="CommitSha">Optionally specify a commit sha to deploy from. If not specified, will deploy from the default branch and track new commits based on the update behavior.</param>
/// <param name="UpdateBehavior"> How to handle updates when a new image digest is detected for the currently defined tags in the compose file.</param>
/// <param name="Webhook">Webhook settings used to deploy this stack from Git provider push events.</param>
/// <param name="RegistryName">The name of the container registry to use for the stack.</param>
public sealed record GitStack(
    Guid GitRepoId,
    string Branch,
    string? CommitSha,
    StackUpdateBehavior UpdateBehavior,
    string? ProjectName = null,
    StackWebhookConfig? Webhook = null,
    List<string>? ComposePaths = null,
    StackCommand? PreDeploy = null,
    StackCommand? PostDeploy = null,
    List<string>? EnvVars = null,
    List<string>? AdditionalEnvFileFromRepo = null,
    string? EnvFilePath = null,
    Guid? RegistryId = null,
    bool DestroyBeforeDeploy = true) : StackSpec(ProjectName, PreDeploy, PostDeploy, EnvVars, EnvFilePath, RegistryId, DestroyBeforeDeploy);

public sealed record StackWebhookConfig(
    bool Enabled = false,
    WebhookProvider Provider = WebhookProvider.GitHub,
    WebhookAuthScheme AuthScheme = WebhookAuthScheme.GitHubHmacSha256,
    string? Secret = null,
    string? BranchFilter = null,
    bool ForceDeploy = false) : WebhookConfig(Enabled, Provider, AuthScheme, Secret, BranchFilter);

public sealed record ImageUpdateState(
    string ServiceName,
    string ImageName,
    string CurrentDigest,
    string? RemoteDigest,
    DateTime LastCheckedAt,
    bool UpdateAvailable
);

public record StackCommand(List<string> Commands, string Path = "./");
