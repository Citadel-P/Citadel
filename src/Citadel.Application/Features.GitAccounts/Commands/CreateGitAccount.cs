using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Git;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using Hosting.Common.Extensions;
using LightResults;
using Mediator;
using Microsoft.AspNetCore.Http;
using System.Security.Claims;

namespace Application.Features.GitAccounts.Commands;

[RequirePermission(ResourceType.GitAccount, PermissionLevel.Write)]
public sealed record CreateGitAccount(
    string Name,
    string Domain,
    GitTransport Transport,
    GitAuthType AuthType,
    GitAuthConfiguration Configuration) : ICommand<Result<GitAccount>>
{
    internal sealed class Validator : AbstractValidator<CreateGitAccount>
    {
        public Validator()
        {
            RuleFor(x => x.Name).ValidNameIdentifier();
            RuleFor(x => x.Domain).NotEmpty()
                .Matches(Validators.RegistryUrlRegex)
                .WithMessage("Please provide a valid host name eg: github.com");
            RuleFor(x => x.Transport).IsInEnum();
            RuleFor(x => x.AuthType).IsInEnum();
            RuleFor(x => x.Configuration).NotNull();
            RuleFor(x => x).Custom((command, context) => ValidateConfiguration(command.Transport, command.AuthType, command.Configuration, context));
        }

        private static void ValidateConfiguration(
            GitTransport transport,
            GitAuthType authType,
            GitAuthConfiguration? configuration,
            ValidationContext<CreateGitAccount> context)
        {
            if (configuration is null)
            {
                context.AddFailure(nameof(CreateGitAccount.Configuration), "Configuration is required.");
                return;
            }

            if (transport == GitTransport.Ssh && configuration is not SshKeyAuth)
                context.AddFailure(nameof(CreateGitAccount.Configuration), "SSH requires SSH key authentication.");

            if (transport != GitTransport.Ssh && configuration is SshKeyAuth)
                context.AddFailure(nameof(CreateGitAccount.Configuration), "SSH auth cannot be used with HTTP/HTTPS.");

            switch (authType, configuration)
            {
                case (GitAuthType.Basic, BasicAuth basic):
                    if (string.IsNullOrWhiteSpace(basic.Username))
                        context.AddFailure(nameof(CreateGitAccount.Configuration), "Basic auth username is required.");
                    if (string.IsNullOrWhiteSpace(basic.Password))
                        context.AddFailure(nameof(CreateGitAccount.Configuration), "Basic auth password is required.");
                    return;
                case (GitAuthType.Token, TokenAuth token):
                    if (string.IsNullOrWhiteSpace(token.Token))
                        context.AddFailure(nameof(CreateGitAccount.Configuration), "Token is required.");
                    return;
                case (GitAuthType.SshKey, SshKeyAuth ssh):
                    if (string.IsNullOrWhiteSpace(ssh.Username))
                        context.AddFailure(nameof(CreateGitAccount.Configuration), "SSH username is required.");
                    if (string.IsNullOrWhiteSpace(ssh.PrivateKey))
                        context.AddFailure(nameof(CreateGitAccount.Configuration), "SSH private key is required.");
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

        var gitAccount = new GitAccount(command.Name, command.Domain, command.Transport, command.AuthType, actorId, command.Configuration);
        await unitOfWork.GitAccounts.AddAsync(gitAccount, cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);

        return gitAccount;
    }
}
