using FluentValidation;
using Hosting.Common;
using Hosting.Common.ErrorTypes;
using Infrastructure.DockerHub;
using Infrastructure.Entities.Registries;
using Infrastructure.EntityFramework;
using Infrastructure.GithubCr;
using LightResults;
using Mediator;
using Microsoft.EntityFrameworkCore;

namespace Application.Features.Images.Queries;

public sealed record GetExternalRepositories(string Name) : IQuery<Result<IEnumerable<IImageRepository>>>
{
    internal class Validator : AbstractValidator<GetExternalRepositories>
    {
        public Validator()
        {
            RuleFor(s => s.Name).NotEmpty().ValidNameIdentifier();
        }
    }
}

internal sealed class GetExternalRepositoriesHander(
    ApplicationDbContext dbContext, 
    IDockerHubApi dockerHub,
    IGithubCrApi githubCrApi) : IQueryHandler<GetExternalRepositories, Result<IEnumerable<IImageRepository>>>
{
    public async ValueTask<Result<IEnumerable<IImageRepository>>> Handle(GetExternalRepositories query, CancellationToken cancellationToken)
    {
        var registry = await dbContext.Registries.AsNoTracking().FirstOrDefaultAsync(s => s.Name == query.Name, cancellationToken);
        if (registry == null) 
        {
            return Result.Failure<IEnumerable<IImageRepository>>(new NotFoundError("The provided registry name does exist"));
        }

        if (registry.Configuration is GitHubRegistry ghCfg) 
        {
            var (packages, errorMessage) = await ghCfg.GetPackages(githubCrApi, cancellationToken);
            if (errorMessage != null)
            {
                return Result.Failure<IEnumerable<IImageRepository>>(new BadRequestError(errorMessage));
            }
            return Result.Success(packages?.Map() ?? []);
        }

        if (registry.Configuration is DockerHubRegistry dhCfg)
        {
            var (repositories, errorMessage) = await dhCfg.GetRepositories(dockerHub, cancellationToken);
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
    internal static IEnumerable<IImageRepository> Map(this IEnumerable<GhcrPackage> packages) => packages.Select(Map);
    internal static IImageRepository Map(GhcrPackage package) => new GitHubPackageResponse()
    {
        Name = package.Name,
        Id = package.Id.ToString(),
        CreatedAt = package.CreatedAt,
        UpdatedAt = package.UpdatedAt,
        Url = package.Url,
        HtmlUrl = package.HtmlUrl,
    };

    internal static IEnumerable<IImageRepository> Map(this IEnumerable<DockerHubRepository> repositories) => repositories.Select(Map);
    internal static IImageRepository Map(DockerHubRepository repository) => new DockerHubRepositoryResponse()
    {
        Name = repository.Name,
        Namespace = repository.Namespace,
        LastUpdated = repository.LastUpdated,
        IsPrivate = repository.IsPrivate,
        PullCount = repository.PullCount
    };
}
