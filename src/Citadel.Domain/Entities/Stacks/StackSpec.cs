using System.Diagnostics.CodeAnalysis;
using System.Text.Json.Serialization;

namespace Domain.Entities.Stacks;

/// <param name="ProjectName">Optionally set a different compose project name. If importing existing stack, this should match the compose project name on your host.</param>
/// <param name="PreDeploy">A list of commands to run before deploying the stack. Useful for things like logging into a private registry, or running a script to generate config files.</param>
/// <param name="PostDeploy">A list of commands to run after deploying the stack. Useful for things like running database migrations, or seeding data.</param>
/// <param name="EnvVars"></param>
/// <param name="EnvFilePath">The path to write the file to, relative to the 'Run Directory'. </param>
[JsonPolymorphic]
[DynamicallyAccessedMembers(DynamicallyAccessedMemberTypes.All)]
[JsonDerivedType(typeof(ManualStack), nameof(StackSource.Manual))]
[JsonDerivedType(typeof(GitStack), nameof(StackSource.Git))]
public abstract record StackSpec(
    string? ProjectName,
    List<string>? PreDeploy,
    List<string>? PostDeploy,
    List<string>? EnvVars = null,
    string? EnvFilePath = null
    );

public sealed record ManualStack(
    string ComposeFile,
    StackUpdateBehavior UpdateBehavior,
    List<string>? EnvVars = null,
    string? EnvFilePath = null,
    string? ProjectName = null,
    List<string>? PreDeploy = null,
    List<string>? PostDeploy = null
    ) : StackSpec(ProjectName, PreDeploy, PostDeploy, EnvVars, EnvFilePath);

/// <param name="GitRepoId">Reference to an existing Repo to attach to, <see cref="Git.GitRepository"/></param>
/// <param name="ProjectName"></param>
/// <param name="ComposePaths">Default to the root of the repository, but can specify subdirectory paths to look for compose files.</param>
/// <param name="PreDeploy"></param>
/// <param name="PostDeploy"></param>
/// <param name="EnvVars"></param>
/// <param name="AdditionalEnvFileFromRepo">Additional env files selected from the Repo.</param>
/// <param name="EnvFilePath"></param>
/// <param name="CommitSha">Optionally specify a commit sha to deploy from. If not specified, will deploy from the default branch and track new commits based on the update behavior.</param>
/// <param name="OnCodeChangeUpdateBehavior"> How to handle updates when a new commit is detected.</param>
/// <param name="OnImageChangeUpdateBehavior"> How to handle updates when a new image digest is detected for the currently defined tags in the compose file.</param>
/// <param name="UpdateMechanism">Whether to check for updates based on a polling mechanism or a WebHook mechanism. Only applicable if OnCodeChangeUpdateBehavior is enabled</param>
/// <param name="WebHookConfig">Config to use when UpdateMechanism is set to Webhook</param>
public sealed record GitStack(
    Guid GitRepoId,
    string? CommitSha, 
    StackUpdateBehavior OnImageChangeUpdateBehavior,
    StackUpdateBehavior OnCodeChangeUpdateBehavior,
    UpdateMechanism UpdateMechanism,
    string? ProjectName = null,
    WebHookConfig? WebHookConfig = null,
    List<string>? ComposePaths = null,
    List<string>? PreDeploy = null,
    List<string>? PostDeploy = null,
    List<string>? EnvVars = null,
    List<string>? AdditionalEnvFileFromRepo = null,
    string? EnvFilePath = null) : StackSpec(ProjectName, PreDeploy, PostDeploy, EnvVars, EnvFilePath);

public sealed record ImageUpdateState(
    string ServiceName,
    string ImageName,
    string CurrentDigest,
    string? RemoteDigest,
    DateTime LastCheckedAt,
    bool UpdateAvailable
);

public sealed record WebHookConfig(WebHookAuthStyle AuthStyle, string Url, string Secret);