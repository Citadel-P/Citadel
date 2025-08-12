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

internal sealed class GetExternalRepositoriesHander(IUnitOfWork unitOfWork, IDockerHubRegistryRepository dockerHubService, IGitHubCrRepository gitHubCrService) 
    : IQueryHandler<GetExternalRepositories, Result<IEnumerable<IImageRepository>>>
{
    public async ValueTask<Result<IEnumerable<IImageRepository>>> Handle(GetExternalRepositories query, CancellationToken cancellationToken)
    {
        var configuration = await unitOfWork.Registries.GetRegistryConfigurationAsync(query.Name, cancellationToken);
        if (configuration == null) 
        {
            return Result.Failure<IEnumerable<IImageRepository>>(new NotFoundError("The provided registry name does not exist"));
        }

        if (configuration is GitHubRegistry ghCfg) 
        {
            var (packages, errorMessage) = await gitHubCrService.GetPackagesAsync(ghCfg, cancellationToken);
            if (errorMessage != null)
            {
                return Result.Failure<IEnumerable<IImageRepository>>(new BadRequestError(errorMessage));
            }
            return Result.Success(packages?.Map() ?? []);
        }

        if (configuration is DockerHubRegistry dhCfg)
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
