using FluentValidation;
using Hosting.Common;
using Hosting.Common.ErrorTypes;
using Domain.Entities.Registries;
using Infrastructure.EntityFramework;
using LightResults;
using Mediator;
using Microsoft.EntityFrameworkCore;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Registries;

namespace Application.Features.Images.Queries;

public sealed record GetDockerHubRepositories(string RegistryName) : IQuery<Result<IEnumerable<DockerHubRepositoryInfo>>>
{
    internal class Validator : AbstractValidator<GetDockerHubRepositories>
    {
        public Validator()
        {
            RuleFor(s => s.RegistryName).ValidNameIdentifier();
        }
    }
}

internal sealed class GetDockerHubRepositoriesHandler(
    ApplicationDbContext dbContext, 
    IDockerHubService dockerHubService) : IQueryHandler<GetDockerHubRepositories, Result<IEnumerable<DockerHubRepositoryInfo>>>
{
    public async ValueTask<Result<IEnumerable<DockerHubRepositoryInfo>>> Handle(GetDockerHubRepositories query, CancellationToken cancellationToken)
    {
        var registry = await dbContext.Registries.AsNoTracking().FirstOrDefaultAsync(s => s.Name == query.RegistryName, cancellationToken);
        if (registry == null)
        {
            return Result.Failure<IEnumerable<DockerHubRepositoryInfo>>(new NotFoundError("The provided registry name does exist"));
        }

        if (registry.Configuration is not DockerHubRegistry cfg)
        {
            return Result.Failure<IEnumerable<DockerHubRepositoryInfo>>(new NotFoundError("The provided registry is not a DockerHub registry instance"));
        }

        var (repos, error) = await dockerHubService.GetRepositoriesAsync(cfg, cancellationToken);
        if (error != null)
        {
            return Result.Failure< IEnumerable<DockerHubRepositoryInfo>>(new BadRequestError(error));
        }

        return repos?.ToList() ?? [];
    }
}