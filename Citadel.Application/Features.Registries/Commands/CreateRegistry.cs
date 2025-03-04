using Infrastructure.Entities;
using Mediator;
using FluentValidation;
using Microsoft.EntityFrameworkCore;
using LightResults;
using Hosting.Common.ErrorTypes;
using Infrastructure;
using Infrastructure.EntityFramework;
using Application.Utils;
using Infrastructure.DockerHub;
using Refit;

namespace Application.Features.Registries.Commands;

public sealed record CreateRegistry(string Name, string Url, RegistryDiscriminator Discriminator, IRegistryConfiguration Configuration) : ICommand<Result<Registry>>
{
    internal sealed class CreateRegistryRequestValidator : AbstractValidator<CreateRegistry>
    {
        public CreateRegistryRequestValidator()
        {
            RuleFor(x => x.Name).NotEmpty().MinimumLength(3);
            RuleFor(x => x.Discriminator)
                .Must(d => Enum.IsDefined(d))
                .WithMessage("'{PropertyName}' must be a valid discriminator");

            When(x => x.Configuration is not DockerHubRegistry, () =>
            {
                RuleFor(x => x.Url).Matches(Constants.Url).WithMessage("Please provide a valid url");
            });
            When(x => x.Configuration is DockerHubRegistry, () =>
            {
                RuleFor(x => x.Configuration as DockerHubRegistry).SetValidator(new DockerHubRegistryValidator());
            });
            When(x => x.Configuration is AzureRegistry, () =>
            {
                RuleFor(x => x.Configuration as AzureRegistry).SetValidator(new AzureRegistryValidator());
            });
            When(x => x.Configuration is AWSRegistry, () =>
            {
                RuleFor(x => x.Configuration as AWSRegistry).SetValidator(new AWSRegistryValidator());
            });
            When(x => x.Configuration is GitlabRegistry, () =>
            {
                RuleFor(x => x.Configuration as GitlabRegistry).SetValidator(new GitlabRegistryValidator());
            });
            When(x => x.Configuration is CustomRegistry, () =>
            {
                RuleFor(x => x.Configuration as CustomRegistry).SetValidator(new CustomRegistryValidator());
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

    internal sealed class CustomRegistryValidator : AbstractValidator<CustomRegistry>
    {
        public CustomRegistryValidator()
        {
            RuleFor(x => x.UserName).NotEmpty().MinimumLength(5);
            RuleFor(x => x.Password).NotEmpty().MinimumLength(6);
        }
    }
}

internal class CreateRegistryHandler(ApplicationDbContext dbContext, IDockerHubApi dockerHub) : ICommandHandler<CreateRegistry, Result<Registry>>
{
    public async ValueTask<Result<Registry>> Handle(CreateRegistry command, CancellationToken cancellationToken)
    {
        // Check for conflicted entries
        Registry registry = await dbContext.Registries.FirstOrDefaultAsync(s => s.Url == command.Url || s.Name == command.Name, cancellationToken);
        if (registry != null)
        {
            return Result.Failure<Registry>(new ConflictError("The provided name or url already exist"));
        }

        if (command.Configuration is DockerHubRegistry cfg)
        {
            try
            {
                var authResponse = await dockerHub.AuthCreateAccessToken(new Body() { Identifier = cfg.UserName, Secret = cfg.PAT }, cancellationToken);

            }
            catch (ApiException ex)
            {
                if (ex.StatusCode == System.Net.HttpStatusCode.Unauthorized)
                {
                    return Result.Failure<Registry>(new BadRequestError("Invalid DockerHub credentials, please check your PAT and/or User-name."));
                }
                return Result.Failure<Registry>(new BadRequestError(ex.Message));
            }
            catch (Exception ex) 
            {
                return Result.Failure<Registry>(new InternalServerError(ex.Message));
            }
        }

        registry = Registry.Create(name: command.Name, url: command.Url, discriminator: command.Discriminator, configuration: command.Configuration);
        
        dbContext.Registries.Add(registry);
        //await dbContext.SaveChangesAsync(cancellationToken);

        return Result.Success(registry);
    }
}