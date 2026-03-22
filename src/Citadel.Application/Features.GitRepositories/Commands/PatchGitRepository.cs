using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Git;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.ErrorTypes;
using Hosting.Common.MergePatch;
using LightResults;
using Mediator;

namespace Application.Features.GitRepositories.Commands;

public sealed record PatchGitRepository(Guid Id, JsonMergePatchDocument<GitRepository> Patch) : ICommand<Result<GitRepository>>
{
    internal sealed class Validator : PatchCommandValidator<PatchGitRepository, GitRepository>
    {
        public Validator()
            : base(
                patchSelector: x => x.Patch,
                jsonTypeInfo: GitJsonContext.Default.GitRepository,
                modelValidator: new GitRepositoryValidator())
        {
        }
    }

    internal sealed class GitRepositoryValidator : AbstractValidator<GitRepository>
    {
        public GitRepositoryValidator()
        {
            RuleFor(x => x.Id).NotEmpty();
            RuleFor(x => x.Name).ValidNameIdentifier();
            RuleFor(x => x.Url).NotEmpty();
            RuleFor(x => x.DefaultBranch).NotEmpty();
            RuleFor(x => x.Status).IsInEnum();
        }
    }
}

internal sealed class PatchGitRepositoryHandler(IUnitOfWork unitOfWork) : ICommandHandler<PatchGitRepository, Result<GitRepository>>
{
    public async ValueTask<Result<GitRepository>> Handle(PatchGitRepository command, CancellationToken cancellationToken)
    {
        var gitRepository = await unitOfWork.GitRepositories.GetAsync(command.Id, cancellationToken);
        if (gitRepository is null)
            return Result.Failure<GitRepository>(new NotFoundError("The provided git repository does not exist"));

        var patchedGitRepository = command.Patch.ApplyTo(gitRepository, GitJsonContext.Default.GitRepository);
        if (!string.Equals(gitRepository.Name, patchedGitRepository.Name, StringComparison.OrdinalIgnoreCase))
        {
            var conflict = await unitOfWork.GitRepositories.ExistsAsync(command.Id, patchedGitRepository.Name, cancellationToken);
            if (conflict)
                return Result.Failure<GitRepository>(new ConflictError("Name already exists"));
        }

        var validation = await GitRepositoryUrlValidation.ValidateAsync(unitOfWork, patchedGitRepository.GitAccountId, patchedGitRepository.Url, cancellationToken);
        if (validation.IsFailure())
            return Result.Failure<GitRepository>(validation.Errors);

        gitRepository.UpdateMetadata(
            patchedGitRepository.Name,
            patchedGitRepository.Description,
            patchedGitRepository.DefaultBranch!,
            patchedGitRepository.Status);
        gitRepository.UpdateSource(patchedGitRepository.Url, patchedGitRepository.GitAccountId);

        await unitOfWork.GitRepositories.UpdateAsync(gitRepository, cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);

        return gitRepository;
    }
}
