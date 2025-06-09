using FluentValidation;
using Hosting.Common;
using Hosting.Common.ErrorTypes;
using Infrastructure.Entities.Registries;
using Infrastructure.EntityFramework;
using Infrastructure.GithubCr;
using LightResults;
using Mediator;
using Microsoft.EntityFrameworkCore;

namespace Application.Features.Images.Queries;

public sealed record GetGithubPackageVersions(string RegistryName, string PackageName) : IQuery<Result<IEnumerable<GhcrPackageVersion>>>
{
    internal class Validator : AbstractValidator<GetGithubPackageVersions>
    {
        public Validator()
        {
            RuleFor(s => s.RegistryName).ValidNameIdentifier();
            RuleFor(s => s.PackageName).NotEmpty().NotNull();
        }
    }
}

internal class GetGithubPackageVersionsHander(ApplicationDbContext dbContext, IGithubCrApi githubCrApi) : IQueryHandler<GetGithubPackageVersions, Result<IEnumerable<GhcrPackageVersion>>>
{
    public async ValueTask<Result<IEnumerable<GhcrPackageVersion>>> Handle(GetGithubPackageVersions query, CancellationToken cancellationToken)
    {
        var registry = await dbContext.Registries.AsNoTracking().FirstOrDefaultAsync(s => s.Name == query.RegistryName, cancellationToken);
        if (registry == null) 
        {
            return Result.Failure<IEnumerable<GhcrPackageVersion>>(new NotFoundError("The provided registry name does exist"));
        }

        if (registry.Configuration is not GitHubRegistry cfg)
        {
            return Result.Failure<IEnumerable<GhcrPackageVersion>>(new NotFoundError("The provided registry is not a Github registry instance"));
        }

        var (versions, errorMessage) = await cfg.GetPackageVersions(githubCrApi, query.PackageName, cancellationToken);
        if (errorMessage != null)
        {
            return Result.Failure<IEnumerable<GhcrPackageVersion>>(new BadRequestError(errorMessage));
        }

        return versions?.ToList() ?? [];
    }
}