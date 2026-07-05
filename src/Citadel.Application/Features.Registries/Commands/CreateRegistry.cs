using Application.Features.Deployments.Notifications;
using Application.Services;
using Application.Services.SignalR;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Registries;
using Domain.Entities.Activities;
using Domain.Entities.Registries;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.Abstraction;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.Registries.Commands;

[RequirePermission(ResourceType.Registry, PermissionLevel.Write)]
public sealed record CreateRegistry(
    string Name, 
    string RegistryHost,
    RegistryStatus Status,
    RegistryConfiguration Configuration, 
    string? Description = null,
    IReadOnlyCollection<Guid>? TagIds = null) : ICommand<Result<Registry>>
{
    internal sealed class Validator : AbstractValidator<CreateRegistry>
    {
        public Validator()
        {
            RuleFor(x => x.Name).ValidNameIdentifier();
            RuleFor(x => x.Description).MaximumLength(600);

            RuleFor(x => x.RegistryHost).NotNull().NotEmpty()
                .Matches(Validators.RegistryUrlRegex).WithMessage("Please provide a valid host name eg: ghcr.io");

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
            When(x => x.Configuration is CustomRegistry, () =>
            {
                RuleFor(x => x.Configuration as CustomRegistry).SetValidator(new CustomRegistryValidator()!);
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

    internal sealed class CustomRegistryValidator : AbstractValidator<CustomRegistry>
    {
        public CustomRegistryValidator()
        {
            When(x => x.AuthEnabled == true, () =>
            {
                RuleFor(x => x.UserName).NotEmpty().MinimumLength(3);
                RuleFor(x => x.Password).NotEmpty().MinimumLength(6);
            });
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
            When(x => x.GhcrAuthEnabled == true, () =>
            {
                RuleFor(x => x.PAT).NotEmpty().MinimumLength(10);
                RuleFor(x => x.NameSpace).NotEmpty().MinimumLength(5);
            });
            
        }
    }
}

internal class CreateRegistryHandler(
    IUnitOfWork unitOfWork, 
    IUserContextAccessor userContext,
    IActivityStreamManager activityHub,
    INotificationQueue notificationQueue,
    IRegistryConnectorResolver registryResolver) : ICommandHandler<CreateRegistry, Result<Registry>>
{
    public async ValueTask<Result<Registry>> Handle(CreateRegistry command, CancellationToken cancellationToken)
    {
        var actorId = userContext.Current.ActorId;
        var exist = await unitOfWork.Registries.ExistsAsync(command.Name, cancellationToken);
        if (exist)
        {
            return Result.Failure<Registry>(new ConflictError("Name already exists"));
        }

        var strategy = registryResolver.Resolve(command.Configuration);
        if (strategy is null)
        {
            return Result.Failure<Registry>(new BadRequestError("Unsupported Registry Type"));
        }

        var (canConnect, errorMessage) = await strategy.CanConnectAsync(command.Configuration, cancellationToken);
        if (!canConnect)
        {
            return Result.Failure<Registry>(new BadRequestError(errorMessage ?? ""));
        }

        var registry = new Registry(
            name: command.Name,
            registryHost: command.RegistryHost, 
            status: command.Status, 
            createdByActorId: actorId, 
            configuration: command.Configuration, 
            description: command.Description);

        var activity = new ActivityEvent(
            actorId: actorId,
            resourceId: registry.Id,
            platformId: null,
            resourceName: registry.Name,
            status: ActivityStatus.Success,
            eventType: ActivityEventType.RegistryCreated,
            info: new RegistryCreated(registry.ToSnapshot())
            );

        await unitOfWork.ActivityEventRepository.AddAsync(activity, cancellationToken);
        await unitOfWork.Registries.AddAsync(registry, cancellationToken, command.TagIds, actorId);
        await unitOfWork.CommitAsync(cancellationToken);

        await notificationQueue.EnqueueAsync(new ActivityNotificationWorkItem(activityHub, await activity.AssignActor(unitOfWork, cancellationToken)), cancellationToken);
        return registry;
    }
}
