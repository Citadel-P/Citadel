using Application.Services.SignalR;
using Application.TaskJobs.WorkItems;
using Domain;
using Hosting.Common;
using Domain.Contracts.Interfaces;
using Domain.Entities.Deployments;
using FluentValidation;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using Hosting.Common.MergePatch;
using LightResults;
using Mediator;

namespace Application.Features.Deployments.Commands;

[RequirePermission(ResourceType.Deployment, ResourceAction.Update)]
public sealed record PatchDeploymentMetadata(Guid Id, JsonMergePatchDocument<Deployment> Patch) : ICommand<Result<Deployment>>
{
    internal sealed class Validator : PatchCommandValidator<PatchDeploymentMetadata, Deployment>
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
            When(s => s.Description != null, () => RuleFor(x => x.Description).MaximumLength(600));
        }
    }
}

internal sealed class PatchDeploymentMetadataHandler(
    IUnitOfWork unitOfWork, 
    IDeploymentStreamManager deploymentHub, 
    INotificationQueue notificationQueue) : ICommandHandler<PatchDeploymentMetadata, Result<Deployment>>
{
    public async ValueTask<Result<Deployment>> Handle(PatchDeploymentMetadata command, CancellationToken cancellationToken)
    {
        var deployment = await unitOfWork.Deployments.GetAsync(command.Id, cancellationToken);
        if (deployment == null)
        {
            return Result.Failure<Deployment>(new NotFoundError("The provided deployment does not exist"));
        }

        var patchedDeployment = command.Patch.ApplyTo(deployment, DeploymentJsonContext.Default.Deployment);
        
        deployment.PartialUpdate(
            description: patchedDeployment.Description);

        await unitOfWork.Deployments.UpdateAsync(deployment, cancellationToken);


        await unitOfWork.CommitAsync(cancellationToken);

        var workItem = new DeploymentNotificationWorkItem(deploymentHub, deployment);

        // Notify
        await notificationQueue.EnqueueAsync(workItem, cancellationToken);
        return deployment;
    }
}