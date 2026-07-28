using Application.Services;
using Domain;
using Domain.Contracts.Resources.Git;
using Hosting.Common;
using Hosting.Common.Attributes;
using LightResults;
using Mediator;

namespace Application.Features.GitRepositories.Queries;

[RequirePermission(ResourceType.GitRepository, PermissionLevel.Read)]
public sealed record ListGitRepositoryDirectory(
    Guid Id,
    string? CommitSha,
    string? Path)
    : IQuery<Result<GitRepositoryDirectoryListing>>;

internal sealed class ListGitRepositoryDirectoryHandler(
    IGitRepositoryContentService contentService)
    : IQueryHandler<ListGitRepositoryDirectory, Result<GitRepositoryDirectoryListing>>
{
    public async ValueTask<Result<GitRepositoryDirectoryListing>> Handle(
        ListGitRepositoryDirectory query,
        CancellationToken cancellationToken)
        => await contentService.ListDirectoryAsync(
            query.Id,
            query.CommitSha,
            query.Path,
            cancellationToken);
}

[RequirePermission(ResourceType.GitRepository, PermissionLevel.Read)]
public sealed record ReadGitRepositoryFile(
    Guid Id,
    string? CommitSha,
    string Path)
    : IQuery<Result<GitRepositoryFileContent>>;

internal sealed class ReadGitRepositoryFileHandler(
    IGitRepositoryContentService contentService)
    : IQueryHandler<ReadGitRepositoryFile, Result<GitRepositoryFileContent>>
{
    public async ValueTask<Result<GitRepositoryFileContent>> Handle(
        ReadGitRepositoryFile query,
        CancellationToken cancellationToken)
        => await contentService.ReadFileAsync(
            query.Id,
            query.CommitSha,
            query.Path,
            cancellationToken);
}

[RequirePermission(ResourceType.GitRepository, PermissionLevel.Read)]
public sealed record CompareGitRepositoryCommits(
    Guid Id,
    string BaseCommitSha,
    string HeadCommitSha)
    : IQuery<Result<GitCommitComparison>>;

internal sealed class CompareGitRepositoryCommitsHandler(
    IGitRepositoryContentService contentService)
    : IQueryHandler<CompareGitRepositoryCommits, Result<GitCommitComparison>>
{
    public async ValueTask<Result<GitCommitComparison>> Handle(
        CompareGitRepositoryCommits query,
        CancellationToken cancellationToken)
        => await contentService.CompareAsync(
            query.Id,
            query.BaseCommitSha,
            query.HeadCommitSha,
            cancellationToken);
}
