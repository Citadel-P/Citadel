using Application.Services.SignalR;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Git;
using FluentValidation;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using Hosting.Common.MergePatch;
using LightResults;
using Mediator;
using Hosting.Common;
using static Application.Features.GitRepositories.Commands.PatchGitRepositoryHandler;

namespace Application.Features.GitRepositories.Commands;

[RequirePermission(ResourceType.GitRepository, ResourceAction.Update)]
public sealed record PatchGitRepositoryMetadata(Guid Id, JsonMergePatchDocument<GitRepository> Patch) : ICommand<Result<GitRepository>>
{
    internal sealed class Validator : PatchCommandValidator<PatchGitRepositoryMetadata, GitRepository>
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
            When(x => x.Description != null, () => RuleFor(x => x.Description).MaximumLength(600));
        }
    }
}

internal sealed class PatchGitRepositoryMetadataHandler(
    IUnitOfWork unitOfWork,
    INotificationQueue notificationQueue,
    IGitRepositoryStreamManager gitRepositoryHub) : ICommandHandler<PatchGitRepositoryMetadata, Result<GitRepository>>
{
    public async ValueTask<Result<GitRepository>> Handle(PatchGitRepositoryMetadata command, CancellationToken cancellationToken)
    {
        var gitRepository = await unitOfWork.GitRepositories.GetAsync(command.Id, cancellationToken);
        if (gitRepository is null)
            return Result.Failure<GitRepository>(new NotFoundError("The provided git repository does not exist"));

        var patchedGitRepository = command.Patch.ApplyTo(gitRepository, GitJsonContext.Default.GitRepository);

        gitRepository.PartialUpdate(
            description: patchedGitRepository.Description);

        await unitOfWork.GitRepositories.UpdateAsync(gitRepository, cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);

        await notificationQueue.EnqueueAsync(new GitRepositoryNotificationWorkItem(gitRepositoryHub, gitRepository), cancellationToken);
        return gitRepository;
    }
}
