using Infrastructure.Entities;
using Mediator;
using FluentValidation;
using Microsoft.EntityFrameworkCore;
using LightResults;
using Hosting.Common.ErrorTypes;
using Infrastructure;
using Infrastructure.Services.Abstractions;
using Infrastructure.EntityFramework;

namespace Application.Features.Registries.Commands;

public sealed record CreateRegistry(string Name, string Url, RegistryDiscriminator Discriminator, IRegistryConfiguration Configuration) : ICommand<Result<Registry>>
{
    internal sealed class CreateRegistryRequestValidator : AbstractValidator<CreateRegistry>
    {
        public CreateRegistryRequestValidator()
        {
            RuleFor(x => x.Name).NotEmpty().MinimumLength(3);

            RuleFor(x => x.Url)
                .Matches(Constants.Url)
                .WithMessage("Please provide a valid url");

            RuleFor(x => x.Discriminator)
                .Must(d => Enum.IsDefined(d))
                .WithMessage("'{PropertyName}' must be a valid discriminator");

            RuleFor(x => x).Custom((request, ctx) =>
            {
                switch (request.Discriminator)
                {
                    case RegistryDiscriminator.Azure:
                        AzureRegistry azureCfg = request.Configuration as AzureRegistry;
                        if (azureCfg is not null)
                        {
                            new AzureRegistryValidator().ValidateAndThrow(azureCfg);
                        }
                        break;

                    case RegistryDiscriminator.DockerHub:
                        DockerHubRegistry dockerHubCfg = request.Configuration as DockerHubRegistry;
                        if (dockerHubCfg is not null)
                        {
                            new DockerHubRegistryValidator().ValidateAndThrow(dockerHubCfg);
                        }
                        break;

                    case RegistryDiscriminator.AWS:
                        AWSRegistry awsCfg = request.Configuration as AWSRegistry;
                        if (awsCfg.AuthenticationRequired && awsCfg is not null)
                        {
                            new AWSRegistryValidator().ValidateAndThrow(awsCfg);
                        }
                        break;

                    case RegistryDiscriminator.Gitlab:
                        GitlabRegistry gitlabCfg = request.Configuration as GitlabRegistry;
                        if (gitlabCfg is not null)
                        {
                            new GitlabRegistryValidator().ValidateAndThrow(gitlabCfg);
                        }
                        break;

                    case RegistryDiscriminator.Custom:
                        CustomRegistry customCfg = request.Configuration as CustomRegistry;
                        if (customCfg.AuthenticationRequired && customCfg is not null)
                        {
                            new CustomRegistryValidator().ValidateAndThrow(customCfg);
                        }
                        break;
                }
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
            RuleFor(x => x.PAT).NotEmpty().MinimumLength(4);
        }
    }

    internal sealed class CustomRegistryValidator : AbstractValidator<CustomRegistry>
    {
        public CustomRegistryValidator()
        {
            RuleFor(x => x.UserName).NotEmpty().MinimumLength(5);
            RuleFor(x => x.Password).NotEmpty().MinimumLength(6);
        }
    }
}

internal class CreateRegistryHandler(ApplicationDbContext dbContext) : ICommandHandler<CreateRegistry, Result<Registry>>
{
    public async ValueTask<Result<Registry>> Handle(CreateRegistry command, CancellationToken cancellationToken)
    {
        // Check for conflict entries
        Registry registry = await dbContext.Registries.FirstOrDefaultAsync(s => s.Url == command.Url || s.Name == command.Name, cancellationToken);
        if (registry != null)
        {
            return Result.Fail<Registry>(new ConflictError("The provided name or url already exist"));
        }

        registry = new Registry()
        {
            Name = command.Name,
            Url = command.Url,
            Created = DateTime.UtcNow,
            Discriminator = command.Discriminator,
            Configuration = command.Configuration.SerializeConfiguration(command.Discriminator)
        };

        await dbContext.Registries.AddAsync(registry, cancellationToken);
        await dbContext.SaveChangesAsync(cancellationToken);

        return Result.Ok(registry);
    }
}