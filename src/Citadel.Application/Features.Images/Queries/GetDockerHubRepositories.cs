using FluentValidation;
using Hosting.Common;
using Hosting.Common.ErrorTypes;
using Domain.Entities.Registries;
using LightResults;
using Mediator;
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
    IUnitOfWork unitOfWork,
    IDockerHubRegistryRepository dockerHubService,
    Application.Permissions.IPermissionEvaluator permissionEvaluator)
    : IQueryHandler<GetDockerHubRepositories, Result<IEnumerable<DockerHubRepositoryInfo>>>
{
    public async ValueTask<Result<IEnumerable<DockerHubRepositoryInfo>>> Handle(GetDockerHubRepositories query, CancellationToken cancellationToken)
    {
        var registry = await unitOfWork.Registries.GetByNameAsync(query.RegistryName, cancellationToken);
        if (registry == null)
        {
            return Result.Failure<IEnumerable<DockerHubRepositoryInfo>>(new NotFoundError("The provided registry name does not exist"));
        }

        var permissions = await permissionEvaluator.EvaluateAsync(registry.Id, ResourceType.Registry, cancellationToken);
        if (!permissions.Has(PermissionLevel.Read))
        {
            return Result.Failure<IEnumerable<DockerHubRepositoryInfo>>(
                new ForbiddenError("Missing permission [Read] on [Registry]."));
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
