using System.Reflection.Metadata.Ecma335;
using System.Text.Json.Serialization;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.ErrorTypes;
using Hosting.Common.Merge;
using Infrastructure;
using Infrastructure.DockerHub;
using Infrastructure.Entities;
using Infrastructure.EntityFramework;
using Infrastructure.EntityFramework.Configurations;
using Infrastructure.GithubCr;
using LightResults;
using Mediator;
using Microsoft.EntityFrameworkCore;

namespace Application.Features.Registries.Commands;

public sealed record PatchRegistry(Guid Id, RegistryDiscriminator Discriminator, string Name, string Url, RegistryConfigurationBase Configuration) : ICommand<Result<Registry>>
{
    internal sealed class PatchRegistryRequestValidator : AbstractValidator<PatchRegistry>
    {
        public PatchRegistryRequestValidator()
        {
            RuleFor(x => x.Id).NotEmpty().NotNull();
            When(s => s.Name != null, () => RuleFor(x => x.Name).NotEmpty().MinimumLength(3));
            RuleFor(x => x.Discriminator)
                .Must(d => Enum.IsDefined(d))
                .WithMessage("'{PropertyName}' must be a valid discriminator");

            When(x => x.Configuration is not DockerHubRegistry && x.Configuration is not GitHubRegistry, () =>
            {
                When(s => s.Name != null, () => RuleFor(x => x.Url).Matches(Validators.UrlRegex).WithMessage("Please provide a valid url"));
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
            When(s => s.UserName != null, () => RuleFor(x => x.UserName).NotEmpty().MinimumLength(3));
            When(s => s.Password != null, () => RuleFor(x => x.Password).NotEmpty().MinimumLength(6));
        }
    }

    internal sealed class DockerHubRegistryValidator : AbstractValidator<DockerHubRegistry>
    {
        public DockerHubRegistryValidator()
        {
            When(s => s.PAT != null, () => RuleFor(x => x.PAT).NotEmpty().MinimumLength(10));
            When(s => s.UserName != null, () => RuleFor(x => x.UserName).NotEmpty().MinimumLength(4));
        }
    }

    internal sealed class AWSRegistryValidator : AbstractValidator<AWSRegistry>
    {
        public AWSRegistryValidator()
        {
            When(s => s.AccessKey != null, () => RuleFor(x => x.AccessKey).NotEmpty().MinimumLength(10));
            When(s => s.SecretAccessKey != null, () => RuleFor(x => x.SecretAccessKey).NotEmpty().MinimumLength(10));
            When(s => s.Region != null, () => RuleFor(x => x.Region).NotEmpty().MinimumLength(4));
        }
    }

    internal sealed class GitlabRegistryValidator : AbstractValidator<GitlabRegistry>
    {
        public GitlabRegistryValidator()
        {
            When(s => s.InstanceUrl != null, () => RuleFor(x => x.InstanceUrl).NotEmpty().MinimumLength(10));
            When(s => s.UserName != null, () => RuleFor(x => x.UserName).NotEmpty().MinimumLength(10));
            When(s => s.PAT != null, () => RuleFor(x => x.PAT).NotEmpty().MinimumLength(10));
        }
    }

    internal sealed class GitHubRegistryValidator : AbstractValidator<GitHubRegistry>
    {
        public GitHubRegistryValidator()
        {
            When(s => s.PAT != null, () => RuleFor(x => x.PAT).NotEmpty().MinimumLength(10));
            When(s => s.Name != null, () => RuleFor(x => x.Name).NotEmpty().MinimumLength(5));
        }
    }
}

internal class PatchRegistryHandler(ApplicationDbContext dbContext, IDockerHubApi dockerHub, IGithubCrApi githubCrApi) : ICommandHandler<PatchRegistry, Result<Registry>>
{
    public async ValueTask<Result<Registry>> Handle(PatchRegistry command, CancellationToken cancellationToken)
    {
        var registry = await dbContext.Registries.FirstOrDefaultAsync(s => s.Id == command.Id, cancellationToken);
        if (registry == null)
        {
            return Result.Failure<Registry>(new ConflictError("The provided Id does not exist"));
        }

        if (command.Name != null) 
        {
            var conflict = await dbContext.Registries.AsNoTracking().FirstOrDefaultAsync(s => s.Name == command.Name && s.Id != command.Id, cancellationToken);
            if (conflict != null) 
            {
                return Result.Failure<Registry>(new ConflictError("A registry with the same name already exist"));
            }
        }

        var patchRegistry = MergeExtensions.Merge(registry, command, RegistryJsonContext.Default.Registry, PatchRegistryJsonContext.Default.PatchRegistry);
        if (patchRegistry == null)
        {
            return Result.Failure<Registry>(new BadRequestError("Failed to apply the patch"));
        }

        if (patchRegistry.Configuration is DockerHubRegistry cfg)
        {
            var (canConnect, errorMessage) = await cfg.CanConnect(dockerHub, cancellationToken);
            if (!canConnect)
            {
                return Result.Failure<Registry>(new BadRequestError(errorMessage ?? ""));
            }
        }
        else if (patchRegistry.Configuration is GitHubRegistry githubRegistry)
        {
            var (canConnect, errorMessage) = await githubRegistry.CanConnect(githubCrApi, cancellationToken);
            if (!canConnect)
            {
                return Result.Failure<Registry>(new BadRequestError(errorMessage));
            }
        }

        registry.PartialUpdate(name: patchRegistry.Name, url: patchRegistry.Url, configuration: patchRegistry.Configuration);
        await dbContext.SaveChangesAsync(cancellationToken);

        return registry;
    }
}

[JsonSerializable(typeof(PatchRegistry))]
internal partial class PatchRegistryJsonContext : JsonSerializerContext
{
}