using Application.Services;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Deployments;
using Domain.Contracts.Resources.Images;
using Domain.Entities;
using Google.Protobuf;
using Mediator;
using Microsoft.Extensions.DependencyInjection;
using System;
using System.Collections.Generic;
using System.Runtime.CompilerServices;
using System.Text;

namespace Application.Features.Deployments.Commands;

public sealed record ApplyDeployment(Guid Id) : IStreamCommand<DeploymentStreamItem>
{
}

internal sealed class ApplyDeploymentHandler(
    IPullImageService pullImageService,
    IPlatformContainerCache platformContainerCache,
    //IConnectorFactory<IDeploymentConnector> connectorFactory,
    IServiceScopeFactory scopeFactory) : IStreamCommandHandler<ApplyDeployment, DeploymentStreamItem>
{
    public async IAsyncEnumerable<DeploymentStreamItem> Handle(ApplyDeployment command, [EnumeratorCancellation] CancellationToken cancellationToken)
    {
        Deployment? deployment;
        await using (var scope = scopeFactory.CreateAsyncScope())
        {
            var uow = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
            deployment = await uow.Deployments.GetAsync(command.Id, cancellationToken);
        }
            
        if (deployment == null )
        {
            var message = $"Deployment with ID {command.Id} not found.";
            yield return new DeploymentStreamItem(ErrorMessage: message, Error: new DeploymentApplyError(404, message));
            yield break;
        }

        if (!platformContainerCache.TryGetCacheEntry(deployment.PlatformId, out var platform, out _))
        {
            var message = $"Platform with ID {deployment.PlatformId} not found or disconnected.";
            yield return new DeploymentStreamItem(ErrorMessage: message, Error: new DeploymentApplyError(404, message));
            yield break;
        }

        if (deployment.Spec?.Image is ExternalImage image)
        {
            yield return new DeploymentStreamItem(ProgressMessage: $"Start Pulling image {image.ImageTag} from the provided registry.");
            await foreach (var item in pullImageService.PullAsync(new PullImageService.PullImageInput(
            ImageTag: image.ImageTag,
            PlatformId: platform.Id,
            RegistryId: image.RegistryId
            ), cancellationToken))
            {
                //yield return item;
            }
            
        }

        //var connector = connectorFactory.GetConnector(platform.ConnectorType);

        yield return new DeploymentStreamItem();
    }
}
