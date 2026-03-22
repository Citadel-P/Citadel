using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Git;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.ErrorTypes;
using Hosting.Common.Extensions;
using LightResults;
using Mediator;
using Microsoft.AspNetCore.Http;
using System.Security.Claims;

namespace Application.Features.GitAccounts.Commands;

public sealed record CreateGitAccount(
    string Name,
    string Domain,
    GitAuthType AuthType,
    GitAccountConfiguration Configuration) : ICommand<Result<GitAccount>>
{
    internal sealed class Validator : AbstractValidator<CreateGitAccount>
    {
        public Validator()
        {
            RuleFor(x => x.Name).ValidNameIdentifier();
            RuleFor(x => x.Domain).NotEmpty()
                .Matches(Validators.RegistryUrlRegex)
                .WithMessage("Please provide a valid host name eg: github.com");
            RuleFor(x => x.AuthType).IsInEnum();
            RuleFor(x => x.Configuration).NotNull();
            RuleFor(x => x).Custom((command, context) => ValidateConfiguration(command.AuthType, command.Configuration, context));
        }

        private static void ValidateConfiguration(
            GitAuthType authType,
            GitAccountConfiguration? configuration,
            ValidationContext<CreateGitAccount> context)
        {
            if (configuration is null)
            {
                context.AddFailure(nameof(CreateGitAccount.Configuration), "Configuration is required.");
                return;
            }

            switch (authType, configuration)
            {
                case (GitAuthType.None, NoAuthAccount):
                    return;
                case (GitAuthType.Ssh, GitSshAccount ssh):
                    if (string.IsNullOrWhiteSpace(ssh.Username))
                        context.AddFailure(nameof(CreateGitAccount.Configuration), "SSH username is required.");
                    if (string.IsNullOrWhiteSpace(ssh.PrivateKey))
                        context.AddFailure(nameof(CreateGitAccount.Configuration), "SSH private key is required.");
                    return;
                case (GitAuthType.Https, GitHttpAccount http):
                    if (http.AuthEnabled == true)
                    {
                        if (string.IsNullOrWhiteSpace(http.Username))
                            context.AddFailure(nameof(CreateGitAccount.Configuration), "HTTP username is required when authentication is enabled.");
                        if (string.IsNullOrWhiteSpace(http.Token))
                            context.AddFailure(nameof(CreateGitAccount.Configuration), "HTTP token is required when authentication is enabled.");
                    }
                    return;
                default:
                    context.AddFailure(nameof(CreateGitAccount.Configuration), $"Configuration type '{configuration.GetType().Name}' does not match auth type '{authType}'.");
                    return;
            }
        }
    }
}

internal sealed class CreateGitAccountHandler(
    IUnitOfWork unitOfWork,
    IHttpContextAccessor httpContextAccessor) : ICommandHandler<CreateGitAccount, Result<GitAccount>>
{
    public async ValueTask<Result<GitAccount>> Handle(CreateGitAccount command, CancellationToken cancellationToken)
    {
        var actorId = httpContextAccessor.HttpContext?.User?.GetActorId()
            ?? throw new ArgumentNullException($"{nameof(ClaimsPrincipal)} is missing");

        var exists = await unitOfWork.GitAccounts.ExistsAsync(command.Name, cancellationToken);
        if (exists)
            return Result.Failure<GitAccount>(new ConflictError("Name already exists"));

        var gitAccount = new GitAccount(command.Name, command.Domain, command.AuthType, actorId, command.Configuration);
        await unitOfWork.GitAccounts.AddAsync(gitAccount, cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);

        return gitAccount;
    }
}
