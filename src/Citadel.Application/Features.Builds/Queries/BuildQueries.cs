using Application.Features.Builds.Models;
using Application.Features.Tags.Queries;
using Domain.Contracts.Interfaces;
using Domain.Entities.Builds;
using Hosting.Common;
using Hosting.Common.Abstraction;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.Builds.Queries;

[RequirePermission(ResourceType.Build, PermissionLevel.Read)]
public sealed record GetBuildProjects(IReadOnlyCollection<string>? Tags = null) : IQuery<Result<BuildProjectListResult>>;

[RequirePermission(ResourceType.Build, PermissionLevel.Read)]
public sealed record GetBuildProject(Guid ProjectId) : IQuery<Result<BuildProjectResult>>;

[RequirePermission(ResourceType.Build, PermissionLevel.Read)]
public sealed record GetBuildRuns(Guid? ProjectId = null, int Limit = 50) : IQuery<Result<BuildRunListResult>>;

[RequirePermission(ResourceType.Build, PermissionLevel.Read)]
public sealed record GetBuildRun(Guid RunId) : IQuery<Result<BuildRunResult>>;

[RequirePermission(ResourceType.Build, PermissionLevel.Read)]
public sealed record GetBuildRunLogs(Guid RunId) : IQuery<Result<BuildRunLogResult>>;

internal sealed class GetBuildProjectsHandler(
    IUnitOfWork unitOfWork,
    IUserContextAccessor userContextAccessor)
    : IQueryHandler<GetBuildProjects, Result<BuildProjectListResult>>
{
    public async ValueTask<Result<BuildProjectListResult>> Handle(GetBuildProjects query, CancellationToken cancellationToken)
    {
        var tagFilter = await TagFilterResolver.ResolveAsync(unitOfWork, query.Tags, cancellationToken);
        if (tagFilter.NoMatch)
            return Result.Success(new BuildProjectListResult([], new Dictionary<Guid, BuildRun>()));

        var user = userContextAccessor.Current;
        var projects = (user is not null && !user.IsAdmin
            ? await unitOfWork.BuildProjects.GetAuthorizedAsync(
                user.UserId,
                ResourceType.Build,
                PermissionLevel.Read,
                SpecificPermission.None,
                cancellationToken,
                tagFilter.TagIds)
            : await unitOfWork.BuildProjects.GetAllAsync(cancellationToken, tagFilter.TagIds)).ToArray();

        var latestRuns = await unitOfWork.BuildRuns.GetLatestByProjectsAsync(
            [.. projects.Select(static project => project.Id)],
            cancellationToken);

        return Result.Success(new BuildProjectListResult(projects, latestRuns));
    }
}

internal sealed class GetBuildProjectHandler(IUnitOfWork unitOfWork)
    : IQueryHandler<GetBuildProject, Result<BuildProjectResult>>
{
    public async ValueTask<Result<BuildProjectResult>> Handle(GetBuildProject query, CancellationToken cancellationToken)
    {
        var project = await unitOfWork.BuildProjects.GetAsync(query.ProjectId, cancellationToken);
        if (project is null)
            return Result.Failure<BuildProjectResult>(new NotFoundError("Build project not found."));

        var latestRun = await unitOfWork.BuildRuns.GetLatestByProjectAsync(project.Id, cancellationToken);
        return Result.Success(new BuildProjectResult(project, latestRun));
    }
}

internal sealed class GetBuildRunsHandler(IUnitOfWork unitOfWork)
    : IQueryHandler<GetBuildRuns, Result<BuildRunListResult>>
{
    public async ValueTask<Result<BuildRunListResult>> Handle(GetBuildRuns query, CancellationToken cancellationToken)
    {
        var runs = query.ProjectId.HasValue
            ? await unitOfWork.BuildRuns.GetByProjectAsync(query.ProjectId.Value, query.Limit, cancellationToken)
            : await unitOfWork.BuildRuns.GetPagedAsync(query.Limit, cancellationToken);

        return Result.Success(new BuildRunListResult([.. runs]));
    }
}

internal sealed class GetBuildRunHandler(IUnitOfWork unitOfWork)
    : IQueryHandler<GetBuildRun, Result<BuildRunResult>>
{
    public async ValueTask<Result<BuildRunResult>> Handle(GetBuildRun query, CancellationToken cancellationToken)
    {
        var run = await unitOfWork.BuildRuns.GetAsync(query.RunId, cancellationToken);
        return run is null
            ? Result.Failure<BuildRunResult>(new NotFoundError("Build run not found."))
            : Result.Success(new BuildRunResult(run));
    }
}

internal sealed class GetBuildRunLogsHandler(IUnitOfWork unitOfWork)
    : IQueryHandler<GetBuildRunLogs, Result<BuildRunLogResult>>
{
    public async ValueTask<Result<BuildRunLogResult>> Handle(GetBuildRunLogs query, CancellationToken cancellationToken)
    {
        var run = await unitOfWork.BuildRuns.GetAsync(query.RunId, cancellationToken);
        if (run is null)
            return Result.Failure<BuildRunLogResult>(new NotFoundError("Build run not found."));

        var logs = await unitOfWork.BuildRunLogs.GetByRunAsync(query.RunId, cancellationToken);
        return Result.Success(new BuildRunLogResult(query.RunId, logs));
    }
}
