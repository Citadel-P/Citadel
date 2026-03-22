using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Git;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.ErrorTypes;
using Hosting.Common.MergePatch;
using LightResults;
using Mediator;

namespace Application.Features.GitAccounts.Commands;

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
            RuleFor(x => x.AuthType).IsInEnum();
            RuleFor(x => x.Configuration).NotNull();
            RuleFor(x => x).Custom((gitAccount, context) => ValidateConfiguration(gitAccount.AuthType, gitAccount.Configuration, context));
        }

        private static void ValidateConfiguration(
            GitAuthType authType,
            GitAccountConfiguration? configuration,
            ValidationContext<GitAccount> context)
        {
            if (configuration is null)
            {
                context.AddFailure(nameof(GitAccount.Configuration), "Configuration is required.");
                return;
            }

            switch (authType, configuration)
            {
                case (GitAuthType.None, NoAuthAccount):
                    return;
                case (GitAuthType.Ssh, GitSshAccount ssh):
                    if (string.IsNullOrWhiteSpace(ssh.Username))
                        context.AddFailure(nameof(GitAccount.Configuration), "SSH username is required.");
                    if (string.IsNullOrWhiteSpace(ssh.PrivateKey))
                        context.AddFailure(nameof(GitAccount.Configuration), "SSH private key is required.");
                    return;
                case (GitAuthType.Https, GitHttpAccount http):
                    if (http.AuthEnabled == true)
                    {
                        if (string.IsNullOrWhiteSpace(http.Username))
                            context.AddFailure(nameof(GitAccount.Configuration), "HTTP username is required when authentication is enabled.");
                        if (string.IsNullOrWhiteSpace(http.Token))
                            context.AddFailure(nameof(GitAccount.Configuration), "HTTP token is required when authentication is enabled.");
                    }
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
            var conflict = await unitOfWork.GitAccounts.ExistsAsync(command.Id, patchedGitAccount.Name, cancellationToken);
            if (conflict)
                return Result.Failure<GitAccount>(new ConflictError("Name already exists"));
        }

        gitAccount.PartialUpdate(
            name: patchedGitAccount.Name,
            domain: patchedGitAccount.Domain,
            authType: patchedGitAccount.AuthType,
            configuration: patchedGitAccount.Configuration);

        await unitOfWork.GitAccounts.UpdateAsync(gitAccount, cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);

        return gitAccount;
    }
}
