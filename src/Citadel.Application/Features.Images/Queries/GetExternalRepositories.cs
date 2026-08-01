using FluentValidation;
using Hosting.Common;
using Hosting.Common.ErrorTypes;
using Domain.Entities.Registries;
using LightResults;
using Mediator;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Registries;

namespace Application.Features.Images.Queries;

public sealed record GetExternalRepositories(string Name) : IQuery<Result<IEnumerable<IImageRepository>>>
{
    internal class Validator : AbstractValidator<GetExternalRepositories>
    {
        public Validator()
        {
            RuleFor(s => s.Name).ValidNameIdentifier();
        }
    }
}

internal sealed class GetExternalRepositoriesHander(
    IUnitOfWork unitOfWork,
    IDockerHubRegistryRepository dockerHubService,
    IGitHubCrRepository gitHubCrService,
    Application.Permissions.IPermissionEvaluator permissionEvaluator)
    : IQueryHandler<GetExternalRepositories, Result<IEnumerable<IImageRepository>>>
{
    public async ValueTask<Result<IEnumerable<IImageRepository>>> Handle(GetExternalRepositories query, CancellationToken cancellationToken)
    {
        var registry = await unitOfWork.Registries.GetByNameAsync(query.Name, cancellationToken);
        if (registry == null) 
        {
            return Result.Failure<IEnumerable<IImageRepository>>(new NotFoundError("The provided registry name does not exist"));
        }

        var permissions = await permissionEvaluator.EvaluateAsync(registry.Id, ResourceType.Registry, cancellationToken);
        if (!permissions.Has(PermissionLevel.Read))
        {
            return Result.Failure<IEnumerable<IImageRepository>>(
                new ForbiddenError("Missing permission [Read] on [Registry]."));
        }

        if (registry.Configuration is GitHubRegistry ghCfg) 
        {
            var (packages, errorMessage) = await gitHubCrService.GetPackagesAsync(ghCfg, cancellationToken);
            if (errorMessage != null)
            {
                return Result.Failure<IEnumerable<IImageRepository>>(new BadRequestError(errorMessage));
            }
            return Result.Success(packages?.Map() ?? []);
        }

        if (registry.Configuration is DockerHubRegistry dhCfg)
        {
            var (repositories, errorMessage) = await dockerHubService.GetRepositoriesAsync(dhCfg, cancellationToken);
            if (errorMessage != null)
            {
                return Result.Failure<IEnumerable<IImageRepository>>(new BadRequestError(errorMessage));
            }
            return Result.Success(repositories?.Map() ?? []);
        }

        return Result.Failure<IEnumerable<IImageRepository>>();
    }
}


internal static partial class Mapper
{
    internal static IEnumerable<IImageRepository> Map(this IEnumerable<GitHubCrPackage> packages) => packages.Select(Map);
    internal static IImageRepository Map(GitHubCrPackage package) => new GitHubPackageResponse()
    {
        Name = package.Name,
        Id = package.Id.ToString(),
        CreatedAt = package.CreatedAt,
        UpdatedAt = package.UpdatedAt,
        Url = package.Url,
        HtmlUrl = package.HtmlUrl,
    };

    internal static IEnumerable<IImageRepository> Map(this IEnumerable<DockerHubRepositoryInfo> repositories) => repositories.Select(Map);
    internal static IImageRepository Map(DockerHubRepositoryInfo repository) => new DockerHubRepositoryResponse()
    {
        Name = repository.Name,
        Namespace = repository.Namespace,
        LastUpdated = repository.LastUpdated,
        IsPrivate = repository.IsPrivate,
        PullCount = repository.PullCount
    };
}
