using Infrastructure.Entities;
using Mediator;
using FluentValidation;
using Microsoft.EntityFrameworkCore;
using LightResults;
using Hosting.Common.ErrorTypes;
using Infrastructure;
using Infrastructure.EntityFramework;
using Infrastructure.DockerHub;
using Infrastructure.GithubCr;
using Hosting.Common;

namespace Application.Features.Registries.Commands;

public sealed record CreateRegistry(string Name, string Url, RegistryDiscriminator Discriminator, RegistryConfigurationBase Configuration) : ICommand<Result<Registry>>
{
    internal sealed class CreateRegistryRequestValidator : AbstractValidator<CreateRegistry>
    {
        public CreateRegistryRequestValidator()
        {
            RuleFor(x => x.Name).NotEmpty().MinimumLength(3);
            RuleFor(x => x.Discriminator)
                .Must(d => Enum.IsDefined(d))
                .WithMessage("'{PropertyName}' must be a valid discriminator");

            When(x => x.Configuration is not DockerHubRegistry && x.Configuration is not GitHubRegistry, () =>
            {
                RuleFor(x => x.Url).Matches(Validators.UrlRegex).WithMessage("Please provide a valid url");
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

internal class CreateRegistryHandler(ApplicationDbContext dbContext, IDockerHubApi dockerHub, IGithubCrApi githubCrApi) : ICommandHandler<CreateRegistry, Result<Registry>>
{
    public async ValueTask<Result<Registry>> Handle(CreateRegistry command, CancellationToken cancellationToken)
    {
        var registry = await dbContext.Registries.FirstOrDefaultAsync(s => s.Name == command.Name, cancellationToken);
        if (registry != null)
        {
            return Result.Failure<Registry>(new ConflictError("The provided name already exist"));
        }

        if (command.Configuration is DockerHubRegistry cfg)
        {
            var (canConnect, errorMessage) = await cfg.CanConnect(dockerHub, cancellationToken);
            if (!canConnect)
            {
                return Result.Failure<Registry>(new BadRequestError(errorMessage ?? ""));
            }
        }
        else if (command.Configuration is GitHubRegistry githubRegistry) 
        {
            var (canConnect, errorMessage) = await githubRegistry.CanConnect(githubCrApi, cancellationToken);
            if (!canConnect)
            {
                return Result.Failure<Registry>(new BadRequestError(errorMessage));
            }
        }

        
        registry = Registry.Create(name: command.Name, url: command.Url, discriminator: command.Discriminator, configuration: command.Configuration);
        dbContext.Registries.Add(registry);
        await dbContext.SaveChangesAsync(cancellationToken);

        return registry;
    }
}