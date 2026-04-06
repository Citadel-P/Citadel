using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Git;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using Hosting.Common.MergePatch;
using LightResults;
using Mediator;

namespace Application.Features.GitAccounts.Commands;

[RequirePermission(ResourceType.GitAccount, ResourceAction.Update)]
public sealed record PatchGitAccount(Guid Id, JsonMergePatchDocument<GitAccount> Patch) : ICommand<Result<GitAccount>>
{
    internal sealed class Validator : PatchCommandValidator<PatchGitAccount, GitAccount>
    {
        public Validator()
            : base(
                patchSelector: x => x.Patch,
                jsonTypeInfo: GitJsonContext.Default.GitAccount,
                modelValidator: new GitAccountValidator())
        {
        }
    }

    internal sealed class GitAccountValidator : AbstractValidator<GitAccount>
    {
        public GitAccountValidator()
        {
            RuleFor(x => x.Id).NotEmpty();
            RuleFor(x => x.Name).ValidNameIdentifier();
            RuleFor(x => x.Domain).NotEmpty()
                .Matches(Validators.RegistryUrlRegex)
                .WithMessage("Please provide a valid host name eg: github.com");
            RuleFor(x => x.Transport).IsInEnum();
            RuleFor(x => x.AuthType).IsInEnum();
            RuleFor(x => x.Configuration).NotNull();
            RuleFor(x => x).Custom((gitAccount, context) => ValidateConfiguration(gitAccount.Transport, gitAccount.AuthType, gitAccount.Configuration, context));
        }

        private static void ValidateConfiguration(
            GitTransport transport,
            GitAuthType authType,
            GitAuthConfiguration? configuration,
            ValidationContext<GitAccount> context)
        {
            if (configuration is null)
            {
                context.AddFailure(nameof(GitAccount.Configuration), "Configuration is required.");
                return;
            }

            if (transport == GitTransport.Ssh && configuration is not SshKeyAuth)
                context.AddFailure(nameof(GitAccount.Configuration), "SSH requires SSH key authentication.");

            if (transport != GitTransport.Ssh && configuration is SshKeyAuth)
                context.AddFailure(nameof(GitAccount.Configuration), "SSH auth cannot be used with HTTP/HTTPS.");

            switch (authType, configuration)
            {
                case (GitAuthType.Basic, BasicAuth basic):
                    if (string.IsNullOrWhiteSpace(basic.Username))
                        context.AddFailure(nameof(GitAccount.Configuration), "Basic auth username is required.");
                    if (string.IsNullOrWhiteSpace(basic.Password))
                        context.AddFailure(nameof(GitAccount.Configuration), "Basic auth password is required.");
                    return;
                case (GitAuthType.Token, TokenAuth token):
                    if (string.IsNullOrWhiteSpace(token.Token))
                        context.AddFailure(nameof(GitAccount.Configuration), "Token is required.");
                    return;
                case (GitAuthType.SshKey, SshKeyAuth ssh):
                    if (string.IsNullOrWhiteSpace(ssh.Username))
                        context.AddFailure(nameof(GitAccount.Configuration), "SSH username is required.");
                    if (string.IsNullOrWhiteSpace(ssh.PrivateKey))
                        context.AddFailure(nameof(GitAccount.Configuration), "SSH private key is required.");
                    return;
                default:
                    context.AddFailure(nameof(GitAccount.Configuration), $"Configuration type '{configuration.GetType().Name}' does not match auth type '{authType}'.");
                    return;
            }
        }
    }
}

internal sealed class PatchGitAccountHandler(IUnitOfWork unitOfWork) : ICommandHandler<PatchGitAccount, Result<GitAccount>>
{
    public async ValueTask<Result<GitAccount>> Handle(PatchGitAccount command, CancellationToken cancellationToken)
    {
        var gitAccount = await unitOfWork.GitAccounts.GetAsync(command.Id, cancellationToken);
        if (gitAccount is null)
            return Result.Failure<GitAccount>(new NotFoundError("The provided git account does not exist"));

        var patchedGitAccount = command.Patch.ApplyTo(gitAccount, GitJsonContext.Default.GitAccount);
        if (!string.Equals(gitAccount.Name, patchedGitAccount.Name, StringComparison.OrdinalIgnoreCase))
        {
            var conflict = await unitOfWork.GitAccounts.IsNameTakenAsync(command.Id, patchedGitAccount.Name, cancellationToken);
            if (conflict)
                return Result.Failure<GitAccount>(new ConflictError("Name already exists"));
        }

        gitAccount.PartialUpdate(
            name: patchedGitAccount.Name,
            domain: patchedGitAccount.Domain,
            transport: patchedGitAccount.Transport,
            authType: patchedGitAccount.AuthType,
            configuration: patchedGitAccount.Configuration);

        await unitOfWork.GitAccounts.UpdateAsync(gitAccount, cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);

        return gitAccount;
    }
}
