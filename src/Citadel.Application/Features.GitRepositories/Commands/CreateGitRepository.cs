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

namespace Application.Features.GitRepositories.Commands;

public sealed record CreateGitRepository(
    string Name,
    string? Description,
    string Url,
    string DefaultBranch,
    GitReposStatus Status,
    Guid? GitAccountId,
    bool WebHookEnabled = false,
    string? WebHookSecret = null,
    IEnumerable<RepoCommand>? OnClone = null,
    IEnumerable<RepoCommand>? OnPull = null) : ICommand<Result<GitRepository>>
{
    internal sealed class Validator : AbstractValidator<CreateGitRepository>
    {
        public Validator()
        {
            RuleFor(x => x.Name).ValidNameIdentifier();
            RuleFor(x => x.Url).NotEmpty();
            RuleFor(x => x.DefaultBranch).NotEmpty();
            RuleFor(x => x.Status).IsInEnum();
        }
    }
}

internal sealed class CreateGitRepositoryHandler(
    IUnitOfWork unitOfWork,
    IHttpContextAccessor httpContextAccessor) : ICommandHandler<CreateGitRepository, Result<GitRepository>>
{
    public async ValueTask<Result<GitRepository>> Handle(CreateGitRepository command, CancellationToken cancellationToken)
    {
        var actorId = httpContextAccessor.HttpContext?.User?.GetActorId()
            ?? throw new ArgumentNullException($"{nameof(ClaimsPrincipal)} is missing");

        var exists = await unitOfWork.GitRepositories.ExistsAsync(command.Name, cancellationToken);
        if (exists)
            return Result.Failure<GitRepository>(new ConflictError("Name already exists"));

        var validation = await GitRepositoryUrlValidation.ValidateAsync(unitOfWork, command.GitAccountId, command.Url, cancellationToken);
        if (validation.IsFailure())
            return Result.Failure<GitRepository>(validation.Errors);

        var gitRepository = new GitRepository(
            command.Name,
            command.Description,
            command.Url,
            command.DefaultBranch,
            command.Status,
            command.GitAccountId,
            actorId,
            command.WebHookEnabled,
            command.WebHookSecret ?? string.Empty,
            command.OnClone?.ToList(),
            command.OnPull?.ToList());
        await unitOfWork.GitRepositories.AddAsync(gitRepository, cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);

        return gitRepository;
    }
}
