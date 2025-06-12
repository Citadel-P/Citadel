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

public sealed record GetDockerHubRepositoryTags(string RegistryName, string RepositoryName) : IQuery<Result<IEnumerable<DockerHubTag>>>
{
    internal class Validator : AbstractValidator<GetDockerHubRepositoryTags>
    {
        public Validator()
        {
            RuleFor(s => s.RegistryName).ValidNameIdentifier();
            RuleFor(s => s.RepositoryName).NotEmpty().NotNull();
        }
    }
}

public sealed class GetDockerHubRepositoryTagsHandler(
    ApplicationDbContext dbContext,
    IDockerHubService dockerHubService)
    : IQueryHandler<GetDockerHubRepositoryTags, Result<IEnumerable<DockerHubTag>>>
{
    public async ValueTask<Result<IEnumerable<DockerHubTag>>> Handle(GetDockerHubRepositoryTags query, CancellationToken cancellationToken)
    {
        var registry = await dbContext.Registries.AsNoTracking().FirstOrDefaultAsync(s => s.Name == query.RegistryName, cancellationToken);
        if (registry == null)
        {
            return Result.Failure<IEnumerable<DockerHubTag>>(new NotFoundError("The provided registry name does exist"));
        }

        if (registry.Configuration is not DockerHubRegistry cfg)
        {
            return Result.Failure<IEnumerable<DockerHubTag>>(new NotFoundError("The provided registry is not a DockerHub registry instance"));
        }

        var (tags, errorMessage) = await dockerHubService.GetRepositoryTagsAsync(cfg, query.RepositoryName, cancellationToken);
        if (errorMessage != null)
        {
            return Result.Failure<IEnumerable<DockerHubTag>>(new BadRequestError(errorMessage));
        }

        return tags?.ToList() ?? [];
    }
}
