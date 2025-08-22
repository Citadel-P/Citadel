using FluentValidation;
using Hosting.Common;
using Hosting.Common.ErrorTypes;
using Domain.Entities.Registries;
using LightResults;
using Mediator;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Registries;

namespace Application.Features.Images.Queries;

public sealed record GetGithubPackageVersions(string RegistryName, string PackageName) : IQuery<Result<IEnumerable<GitHubCrPackageVersion>>>
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

internal class GetGithubPackageVersionsHander(IUnitOfWork unitOfWork, IGitHubCrRepository gitHubCrService) 
    : IQueryHandler<GetGithubPackageVersions, Result<IEnumerable<GitHubCrPackageVersion>>>
{
    public async ValueTask<Result<IEnumerable<GitHubCrPackageVersion>>> Handle(GetGithubPackageVersions query, CancellationToken cancellationToken)
    {
        var registry = await unitOfWork.Registries.GetByNameAsync(query.RegistryName, cancellationToken);
        if (registry == null) 
        {
            return Result.Failure<IEnumerable<GitHubCrPackageVersion>>(new NotFoundError("The provided registry name does not exist"));
        }

        if (registry.Configuration is not GitHubRegistry cfg)
        {
            return Result.Failure<IEnumerable<GitHubCrPackageVersion>>(new NotFoundError("The provided registry is not a Github registry instance"));
        }

        var (versions, errorMessage) = await gitHubCrService.GetPackageVersionsAsync(cfg, query.PackageName, cancellationToken);
        if (errorMessage != null)
        {
            return Result.Failure<IEnumerable<GitHubCrPackageVersion>>(new BadRequestError(errorMessage));
        }

        return versions?.ToList() ?? [];
    }
}