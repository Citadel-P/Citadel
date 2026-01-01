using Application.Services;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Deployments;
using Domain.Entities;
using Mediator;
using Microsoft.Extensions.DependencyInjection;
using System.Runtime.CompilerServices;

namespace Application.Features.Deployments.Commands;

public sealed record ApplyDeployment(Guid Id) : IStreamCommand<DeploymentStreamItem>;

internal sealed class ApplyDeploymentHandler(
    IPullImageService pullImageService,
    IPlatformContainerCache platformContainerCache,
    IConnectorFactory<IDeploymentConnector> connectorFactory,
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
            var message = $"❌ Deployment with ID {command.Id} not found.";
            yield return new DeploymentStreamItem(ErrorMessage: message, Error: new DeploymentApplyError(404, message));
            yield break;
        }

        if (!platformContainerCache.TryGetCacheEntry(deployment.PlatformId, out var platform, out _))
        {
            var message = $"❌ Platform with ID {deployment.PlatformId} not found or disconnected.";
            yield return new DeploymentStreamItem(ErrorMessage: message, Error: new DeploymentApplyError(404, message));
            yield break;
        }

        if (deployment.Spec is null)
        {
            var message = "❌ Deployment Spec not found";
            yield return new DeploymentStreamItem(ErrorMessage: message, Error: new DeploymentApplyError(400, message));
            yield break;
        }

        string? imageId = null;
        if (deployment.Spec?.Image is ExternalImage image)
        {
            yield return new DeploymentStreamItem(ProgressMessage: $"⏳ Pulling image {image.ImageTag} from the provided registry.");
            await foreach (var item in pullImageService.PullAsync(new PullImageService.PullImageInput(
            ImageTag: image.ImageTag,
            PlatformId: platform.Id,
            RegistryId: image.RegistryId
            ), cancellationToken))
            {
                yield return new DeploymentStreamItem(
                    Status: item.Status,
                    ProgressMessage: item.ProgressMessage,
                    ErrorMessage: item.ErrorMessage,
                    Error: item.Error is not null ? new DeploymentApplyError(item.Error.Code, item.Error.Message) : null
                    );
                if (!string.IsNullOrEmpty(item.ErrorMessage))
                {
                    // TODO: log the error to the audit table
                    yield break;
                }

                // Capture the image ID once pulled
                if (!string.IsNullOrEmpty(item.DockerImageId))
                {
                    imageId = item.DockerImageId;
                }
            }
        }
        else
        {
            imageId = (deployment.Spec?.Image as LocalImage)?.ImageId;
        }

        if (string.IsNullOrEmpty(imageId))
        {
            var message = "❌ Image ID could not be determined for the deployment.";
            yield return new DeploymentStreamItem(ErrorMessage: message, Error: new DeploymentApplyError(400, message));
            yield break;
        }

        yield return new DeploymentStreamItem(ProgressMessage: $"Applying deployment to platform {platform.Address}...");

        var resourceSpec = deployment.Spec.ResourceSpec with
        {
            CpuLimit = deployment.Spec?.ResourceSpec?.CpuLimit.HasValue == true ? (deployment.Spec.ResourceSpec?.CpuLimit.Value * 100000) : 0,
            MemoryLimit = deployment.Spec?.ResourceSpec?.MemoryLimit.HasValue == true ? deployment.Spec?.ResourceSpec?.MemoryLimit * 1024 * 1024 : 0
        };

        var spec = new ApplyDeploymentCommand(
                PlatformAddress: platform.Address,
                Name: deployment.Name,
                ImageId: imageId,
                Spec: deployment.Spec with
                {
                    ResourceSpec = resourceSpec
                });

        var connector = connectorFactory.GetConnector(platform.ConnectorType);
        var result = await connector.ApplyDeploymentAsync(spec, cancellationToken);
        if (!result.IsSuccess(out var containerId, out var error))
        {
            yield return new DeploymentStreamItem(ErrorMessage: $"❌ {error.Message}");
            yield break;
        }

        yield return new DeploymentStreamItem(ProgressMessage: $"Container started: {containerId}.");

        // Todo: link deployment to the container

        yield return new DeploymentStreamItem(ProgressMessage: "✅ Deployment is now running.");
    }
}
