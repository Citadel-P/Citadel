using Application.Services;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities;
using Domain.Entities.Registries;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.Registries.Commands;

[RequirePermission(nameof(AppPermission.Registry_Create))]
public sealed record CreateRegistry(string Name, string Url, RegistryType Type, RegistryConfigurationBase Configuration) : ICommand<Result<Registry>>
{
    internal sealed class Validator : AbstractValidator<CreateRegistry>
    {
        public Validator()
        {
            RuleFor(x => x.Name).NotEmpty().MinimumLength(3);
            RuleFor(x => x.Type)
                .Must(d => Enum.IsDefined(d))
                .WithMessage("'{PropertyName}' must be a valid type");

            When(x => x.Configuration is not DockerHubRegistry && x.Configuration is not GitHubRegistry, () =>
            {
                RuleFor(x => x.Url).Matches(Validators.UrlRegex).WithMessage("Please provide a valid URL");
            });
            When(x => x.Configuration is DockerHubRegistry, () =>
            {
                RuleFor(x => x.Configuration as DockerHubRegistry).SetValidator(new DockerHubRegistryValidator()!);
            });
            When(x => x.Configuration is AzureRegistry, () =>
            {
                RuleFor(x => x.Configuration as AzureRegistry).SetValidator(new AzureRegistryValidator()!);
            });
            When(x => x.Configuration is AWSRegistry, () =>
            {
                RuleFor(x => x.Configuration as AWSRegistry).SetValidator(new AWSRegistryValidator()!);
            });
            When(x => x.Configuration is GitlabRegistry, () =>
            {
                RuleFor(x => x.Configuration as GitlabRegistry).SetValidator(new GitlabRegistryValidator()!);
            });
            When(x => x.Configuration is GitHubRegistry, () =>
            {
                RuleFor(x => x.Configuration as GitHubRegistry).SetValidator(new GitHubRegistryValidator()!);
            });
        }
    }

    internal sealed class AzureRegistryValidator : AbstractValidator<AzureRegistry>
    {
        public AzureRegistryValidator()
        {
            RuleFor(x => x.UserName).NotEmpty().MinimumLength(3);
            RuleFor(x => x.Password).NotEmpty().MinimumLength(6);
        }
    }

    internal sealed class DockerHubRegistryValidator : AbstractValidator<DockerHubRegistry>
    {
        public DockerHubRegistryValidator()
        {
            RuleFor(x => x.PAT).NotEmpty().MinimumLength(10);
            RuleFor(x => x.UserName).NotEmpty().MinimumLength(4);
        }
    }

    internal sealed class AWSRegistryValidator : AbstractValidator<AWSRegistry>
    {
        public AWSRegistryValidator()
        {
            RuleFor(x => x.AccessKey).NotEmpty().MinimumLength(10);
            RuleFor(x => x.SecretAccessKey).NotEmpty().MinimumLength(10);
            RuleFor(x => x.Region).NotEmpty().MinimumLength(4);
        }
    }

    internal sealed class GitlabRegistryValidator : AbstractValidator<GitlabRegistry>
    {
        public GitlabRegistryValidator()
        {
            RuleFor(x => x.InstanceUrl).NotEmpty().MinimumLength(10);
            RuleFor(x => x.UserName).NotEmpty().MinimumLength(10);
            RuleFor(x => x.PAT).NotEmpty().MinimumLength(10);
        }
    }

    internal sealed class GitHubRegistryValidator : AbstractValidator<GitHubRegistry>
    {
        public GitHubRegistryValidator()
        {
            RuleFor(x => x.PAT).NotEmpty().MinimumLength(10);
            RuleFor(x => x.Name).NotEmpty().MinimumLength(5);
        }
    }
}

internal class CreateRegistryHandler(IUnitOfWork unitOfWork, IRegistryConnectorResolver registryResolver) : ICommandHandler<CreateRegistry, Result<Registry>>
{
    public async ValueTask<Result<Registry>> Handle(CreateRegistry command, CancellationToken cancellationToken)
    {
        var exist = await unitOfWork.Registries.ExistsAsync(command.Name, cancellationToken);
        if (exist)
        {
            return Result.Failure<Registry>(new ConflictError("Name already exists"));
        }

        var strategy = registryResolver.Resolve(command.Type);
        if (strategy is null)
        {
            return Result.Failure<Registry>(new BadRequestError("Unsupported Registry Type"));
        }

        var (canConnect, errorMessage) = await strategy.CanConnectAsync(command.Configuration, cancellationToken);
        if (!canConnect)
        {
            return Result.Failure<Registry>(new BadRequestError(errorMessage ?? ""));
        }

        var registry = new Registry(name: command.Name, url: command.Url, type: command.Type, configuration: command.Configuration);
        await unitOfWork.Registries.AddAsync(registry, cancellationToken);
        await unitOfWork.CommitAsync();

        return registry;
    }
}