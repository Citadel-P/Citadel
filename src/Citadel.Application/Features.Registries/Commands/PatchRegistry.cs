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
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using Hosting.Common.Extensions;
using Hosting.Common.MergePatch;
using LightResults;
using Mediator;
using Microsoft.AspNetCore.Http;
using System.Security.Claims;
namespace Application.Features.Registries.Commands;

[RequirePermission(nameof(AppPermission.Registry_Update))]
public sealed record PatchRegistry(Guid Id, JsonMergePatchDocument<Registry> Patch) : ICommand<Result<Registry>>
{
    internal sealed class Validator : PatchCommandValidator<PatchRegistry, Registry>
    {
        public Validator()
            : base(
                  patchSelector: x => x.Patch,
                  jsonTypeInfo: RegistryJsonContext.Default.Registry,
                  modelValidator: new RegistryValidator()
                  ) {}
    }

    internal sealed class RegistryValidator : AbstractValidator<Registry>
    {
        public RegistryValidator()
        {
            RuleFor(x => x.Id).NotEmpty().NotNull();
            When(s => s.Name != null, () => RuleFor(x => x.Name).ValidNameIdentifier());
            When(s => s.Name != null, () => RuleFor(x => x.Description).MaximumLength(600));

            When(s => s.Name != null, () => RuleFor(x => x.RegistryHost).Matches(Validators.RegistryUrlRegex).WithMessage("Please provide a valid host name eg: ghcr.io"));

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
            When(x => x.GhcrAuthEnabled == true, () =>
            {
                When(s => s.PAT != null, () => RuleFor(x => x.PAT).NotEmpty().MinimumLength(10));
                When(s => s.NameSpace!= null, () => RuleFor(x => x.NameSpace).NotEmpty().MinimumLength(5));
            });
            
        }
    }

    internal sealed class CustomRegistryValidator : AbstractValidator<CustomRegistry>
    {
        public CustomRegistryValidator()
        {
            When(x => x.AuthEnabled == true, () =>
            {
                When(s => s.UserName != null, () => RuleFor(x => x.UserName).NotEmpty().MinimumLength(3));
                When(s => s.Password != null, () => RuleFor(x => x.Password).NotEmpty().MinimumLength(6));
            });
        }
    }
}

internal class PatchRegistryHandler(
    IUnitOfWork unitOfWork,
    IActivityStreamManager activityHub,
    INotificationQueue notificationQueue,
    IRegistryConnectorResolver registryResolver,
    IHttpContextAccessor httpContextAccessor) : ICommandHandler<PatchRegistry, Result<Registry>>
{
    public async ValueTask<Result<Registry>> Handle(PatchRegistry command, CancellationToken cancellationToken)
    {
        var actorId = httpContextAccessor.HttpContext?.User?.GetActorId()
           ?? throw new ArgumentNullException($"{nameof(ClaimsPrincipal)} is missing");

        var registry = await unitOfWork.Registries.GetAsync(command.Id, cancellationToken);
        if (registry == null)
        {
            return Result.Failure<Registry>(new NotFoundError("The provided registry does not exist"));
        }

        var patchedRegistry = command.Patch.ApplyTo(registry, RegistryJsonContext.Default.Registry);
        if (patchedRegistry.Name != null) 
        {
            var conflict = await unitOfWork.Registries.ExistsAsync(command.Id, patchedRegistry.Name, cancellationToken);
            if (conflict) 
            {
                return Result.Failure<Registry>(new ConflictError("Name already exists"));
            }
        }

        var strategy = registryResolver.Resolve(patchedRegistry.Configuration);
        if (strategy is null)
        {
            return Result.Failure<Registry>(new BadRequestError("Unsupported Registry Type"));
        }

        var (canConnect, errorMessage) = await strategy.CanConnectAsync(patchedRegistry.Configuration, cancellationToken);
        if (!canConnect)
        {
            return Result.Failure<Registry>(new BadRequestError(errorMessage ?? ""));
        }

        ActivityEvent? activity = null;
        if (!string.IsNullOrEmpty(patchedRegistry.Name) && string.Compare(registry.Name, patchedRegistry.Name, StringComparison.OrdinalIgnoreCase) != 0)
        {
            activity = new ActivityEvent(
               actorId: actorId,
               resourceId: registry.Id,
               platformId: null,
               resourceName: registry.Name,
               status: ActivityStatus.Success,
               eventType: ActivityEventType.RegistryRenamed,
               info: new RegistryRenamed(registry.Name, patchedRegistry.Name)
           );
        }
        else
        {
            activity = new ActivityEvent(
               actorId: actorId,
               resourceId: registry.Id,
               platformId: null,
               resourceName: registry.Name,
               status: ActivityStatus.Success,
               eventType: ActivityEventType.RegistryUpdated,
               info: new RegistryUpdated(registry.ToSnapshot(command.Id), patchedRegistry.ToSnapshot(command.Id))
           );
        }

        registry.PartialUpdate(name: patchedRegistry.Name, registryHost: patchedRegistry.RegistryHost, status: patchedRegistry.Status, 
            description: patchedRegistry.Description, configuration: patchedRegistry.Configuration);

        await unitOfWork.ActivityEventRepository.AddAsync(activity, cancellationToken);
        await unitOfWork.Registries.UpdateAsync(registry, cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);

        await notificationQueue.EnqueueAsync(new ActivityNotificationWorkItem(activityHub, await activity.AssignActor(unitOfWork, cancellationToken)), cancellationToken);
        return registry;
    }
}