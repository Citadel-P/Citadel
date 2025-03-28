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

public sealed record GetDockerHubRepositoryTags(string RegistryName, string RepositoryName) : IQuery<Result<IEnumerable<Tag>>>
{
    internal class Validator : AbstractValidator<GetDockerHubRepositoryTags>
    {
        public Validator()
        {
            RuleFor(s => s.RegistryName).NotEmpty().ValidNameIdentifier();
            RuleFor(s => s.RepositoryName).NotEmpty().NotNull();
        }
    }
}

public sealed class GetDockerHubRepositoryTagsHandler(
    ApplicationDbContext dbContext,
    IDockerHubApi dockerHub)
    : IQueryHandler<GetDockerHubRepositoryTags, Result<IEnumerable<Tag>>>
{
    public async ValueTask<Result<IEnumerable<Tag>>> Handle(GetDockerHubRepositoryTags query, CancellationToken cancellationToken)
    {
        var registry = await dbContext.Registries.AsNoTracking().FirstOrDefaultAsync(s => s.Name == query.RegistryName, cancellationToken);
        if (registry == null)
        {
            return Result.Failure<IEnumerable<Tag>>(new NotFoundError("The provided registry name does exist"));
        }

        if (registry.Configuration is not DockerHubRegistry cfg)
        {
            return Result.Failure<IEnumerable<Tag>>(new NotFoundError("The provided registry is not a DockerHub registry instance"));
        }

        var (tags, errorMessage) = await cfg.GetRepositoryTags(dockerHub, query.RepositoryName, cancellationToken);
        if (errorMessage != null)
        {
            return Result.Failure<IEnumerable<Tag>>(new BadRequestError(errorMessage));
        }

        return tags.ToList();
    }
}
