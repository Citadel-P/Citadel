using FluentValidation;
using Hosting.Common;
using Hosting.Common.ErrorTypes;
using Infrastructure.DockerHub;
using Infrastructure.Entities;
using Infrastructure.EntityFramework;
using LightResults;
using Mediator;
using Microsoft.EntityFrameworkCore;

namespace Application.Features.Images.Queries;

public sealed record GetDockerHubRepositories(string RegistryName) : IQuery<Result<IEnumerable<DockerHubRepository>>>
{
    internal class Validator : AbstractValidator<GetDockerHubRepositories>
    {
        public Validator()
        {
            RuleFor(s => s.RegistryName).NotEmpty().ValidNameIdentifier();
        }
    }
}

internal sealed class GetDockerHubRepositoriesHandler(
    ApplicationDbContext dbContext, 
    IDockerHubApi dockerHub) : IQueryHandler<GetDockerHubRepositories, Result<IEnumerable<DockerHubRepository>>>
{
    public async ValueTask<Result<IEnumerable<DockerHubRepository>>> Handle(GetDockerHubRepositories query, CancellationToken cancellationToken)
    {
        var registry = await dbContext.Registries.AsNoTracking().FirstOrDefaultAsync(s => s.Name == query.RegistryName, cancellationToken);
        if (registry == null)
        {
            return Result.Failure<IEnumerable<DockerHubRepository>>(new NotFoundError("The provided registry name does exist"));
        }

        if (registry.Configuration is not DockerHubRegistry cfg)
        {
            return Result.Failure<IEnumerable<DockerHubRepository>>(new NotFoundError("The provided registry is not a DockerHub registry instance"));
        }

        var (repos, error) = await cfg.GetRepositories(dockerHub, cancellationToken);
        if (error != null)
        {
            return Result.Failure< IEnumerable<DockerHubRepository>>(new BadRequestError(error));
        }

        return repos?.ToList() ?? [];
    }
}