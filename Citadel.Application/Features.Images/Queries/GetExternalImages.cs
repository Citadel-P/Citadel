using FluentValidation;
using Hosting.Common;
using Hosting.Common.ErrorTypes;
using Infrastructure.DockerHub;
using Infrastructure.Entities;
using Infrastructure.EntityFramework;
using Infrastructure.GithubCr;
using LightResults;
using Mediator;
using Microsoft.EntityFrameworkCore;

namespace Application.Features.Images.Queries;

public sealed record GetExternalImages(string Name) : IQuery<Result<IEnumerable<IImageResponse>>>
{
    internal class Validator : AbstractValidator<GetExternalImages>
    {
        public Validator()
        {
            RuleFor(s => s.Name).NotEmpty().ValidNameIdentifier();
        }
    }
}

internal class GetExternalImagesHander(ApplicationDbContext dbContext, IDockerHubApi dockerHub, IGithubCrApi githubCrApi) : IQueryHandler<GetExternalImages, Result<IEnumerable<IImageResponse>>>
{
    public async ValueTask<Result<IEnumerable<IImageResponse>>> Handle(GetExternalImages query, CancellationToken cancellationToken)
    {
        var registry = await dbContext.Registries.AsNoTracking().FirstOrDefaultAsync(s => s.Name == query.Name, cancellationToken);
        if (registry == null) 
        {
            return Result.Failure<IEnumerable<IImageResponse>>(new NotFoundError("The provided registry name does exist"));
        }

        if (registry.Configuration is GitHubRegistry cfg) 
        {
            var result = await cfg.GetPackages(githubCrApi, cancellationToken);
            if (result.errorMessage != null)
            {
                return Result.Failure<IEnumerable<IImageResponse>>(new BadRequestError(result.errorMessage));
            }
            return Result.Success(result.packages.Map());
        }

        return Result.Failure<IEnumerable<IImageResponse>>();
    }
}


internal static partial class Mapper
{
    internal static IEnumerable<IImageResponse> Map(this IEnumerable<GhcrPackage> images) => images.Select(Map);
    internal static IImageResponse Map(GhcrPackage image) => new GitHubPackageResponse()
    {
        Name = image.Name,
        Id = image.Id.ToString(),
        CreatedAt = image.CreatedAt,
        UpdatedAt = image.UpdatedAt,
        Url = image.Url,
    };
}
