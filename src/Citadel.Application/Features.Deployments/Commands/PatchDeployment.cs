using Application.Services.SignalR;
using Application.TaskJobs.WorkItems;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using Hosting.Common.MergePatch;
using LightResults;
using Mediator;

namespace Application.Features.Deployments.Commands;

[RequirePermission(nameof(AppPermission.Deploymen_Update))]
public sealed record PatchDeployment(Guid Id, JsonMergePatchDocument<Deployment> Patch) : ICommand<Result<Deployment>>
{
    internal sealed class Validator : PatchCommandValidator<PatchDeployment, Deployment>
    {
        public Validator()
            : base(
                  patchSelector: x => x.Patch,
                  jsonTypeInfo: DeploymentJsonContext.Default.Deployment,
                  modelValidator: new DeploymentValidator()
                  )
        { }
    }

    internal sealed class DeploymentValidator : AbstractValidator<Deployment>
    {
        public DeploymentValidator()
        {

            RuleFor(x => x.Id).NotEmpty().NotNull();
            When(s => s.Name != null, () => RuleFor(x => x.Name).NotEmpty().ValidNameIdentifier());
            When(s => s.Name != null, () => RuleFor(x => x.Description).MaximumLength(600));
        }
    }
}

internal sealed class PatchDeploymentHandler(IUnitOfWork unitOfWork, IDeploymentStreamManager deploymentHub, INotificationQueue notificationQueue) : ICommandHandler<PatchDeployment, Result<Deployment>>
{
    public async ValueTask<Result<Deployment>> Handle(PatchDeployment command, CancellationToken cancellationToken)
    {
        var deployment = await unitOfWork.Deployments.GetAsync(command.Id, cancellationToken);
        if (deployment == null)
        {
            return Result.Failure<Deployment>(new NotFoundError("The provided deployment does not exist"));
        }

        var patchedDeployment = command.Patch.ApplyTo(deployment, DeploymentJsonContext.Default.Deployment);
        if (patchedDeployment.Name != null)
        {
            var conflict = await unitOfWork.Deployments.ExistsAsync(command.Id, patchedDeployment.Name, cancellationToken);
            if (conflict)
            {
                return Result.Failure<Deployment>(new ConflictError("Name already exists"));
            }
        }

        if (patchedDeployment.Spec?.Image is not ExternalImage)
        {
            if (patchedDeployment.UpdateBehavior != UpdateBehavior.Disabled)
            {
                return Result.Failure<Deployment>(new BadRequestError("Auto-update requires an external image source."));
            }
        }

        if (patchedDeployment.Spec?.Image is ExternalImage extImage && extImage.ImageTag.Contains('@'))
        {
            if (patchedDeployment.UpdateBehavior != UpdateBehavior.Disabled)
            {
                return Result.Failure<Deployment>(new BadRequestError("Cannot enable Auto-update for an image pinned by digest (contains '@')."));
            }
        }

        deployment.PartialUpdate(
            name: patchedDeployment.Name, 
            platformId: patchedDeployment.PlatformId,
            description: patchedDeployment.Description,
            updateBehavior: patchedDeployment.UpdateBehavior,
            spec: patchedDeployment.Spec);

        await unitOfWork.Deployments.UpdateAsync(deployment, cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);

        var workItem = new DeploymentNotificationWorkItem(deploymentHub, deployment);
        await notificationQueue.EnqueueAsync(workItem, cancellationToken);

        return deployment;
    }
}
