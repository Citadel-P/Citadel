using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Images;
using Domain.Contracts.Resources.Networks;
using Domain.Contracts.Resources.Volumes;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.Images.Queries;

public sealed record GetImageInfo(Guid PlatformId, string ImageId) : IQuery<Result<ImageInfoResult>>
{
    internal class Validator : AbstractValidator<InspectImage>
    {
        public Validator()
        {
            RuleFor(s => s.PlatformId).NotEmpty().NotNull();
            RuleFor(s => s.ImageId).ValidHashId();
        }
    }
}

internal sealed class GetImageInfoHandler(IUnitOfWork unitOfWork, 
    IConnectorFactory<IImageConnector> imgConnectorFactory, 
    IConnectorFactory<INetworkConnector> networkConnectorFactory,
    IConnectorFactory<IVolumeConnector> volumeConnectorFactory) 
    : IQueryHandler<GetImageInfo, Result<ImageInfoResult>>
{
    public async ValueTask<Result<ImageInfoResult>> Handle(GetImageInfo query, CancellationToken cancellationToken)
    {
        var platform = await unitOfWork.Platforms.GetByIdAsync(query.PlatformId, cancellationToken);
        if (platform is null)
        {
            return Result.Failure<ImageInfoResult>(new NotFoundError("Platform not found."));
        }
        
        var exposedPortsTask = GetExposedPorts(platform.Address, platform.ConnectorType, query.ImageId, cancellationToken);
        var networksTask = GetNetworks(platform.Address, platform.ConnectorType, cancellationToken);
        var volumesTask = GetVolumes(platform.Address, platform.ConnectorType, cancellationToken);
        var networks = await networksTask;
        var volumes = await volumesTask;
        var exposedPorts = await exposedPortsTask;

        if (networks is null || exposedPorts is null || volumes is null)
        {
            return Result.Failure<ImageInfoResult>(new NotFoundError("Failed to retrieve image information."));
        }

        return Result.Success(new ImageInfoResult
        (
            Volumes: volumes,
            Networks: networks,
            ExposedPorts: exposedPorts,
            MemTotal: platform.MemTotal,
            CpuCount: platform.CpuCount
        ));
    }

    private async Task<IEnumerable<string>?> GetExposedPorts(string platformAddress, PlatformConnectorType connectorType, string imageId, CancellationToken cancellationToken)
    {
        var args = new InspectImageCommand
        (
            PlatformAddress: platformAddress,
            ImageId: imageId
        );
        var inspectResult = await imgConnectorFactory.GetConnector(connectorType).InspectImageAsync(args, cancellationToken: cancellationToken);

        if (inspectResult.IsSuccess(out var inspect))
        {
            return inspect.ExposedPorts;
        }
        return null;
    }

    private async Task<IEnumerable<string>?> GetNetworks(string platformAddress, PlatformConnectorType connectorType, CancellationToken cancellationToken)
    {
        var args = new ListNetworksCommand
        (
            PlatformAddress: platformAddress
        );

        var networksResult = await networkConnectorFactory.GetConnector(connectorType).ListNetworksAsync(args, cancellationToken);
        if (networksResult.IsSuccess(out var networks))
        {
            return networks.Aggregate(new List<string>(), (acc, network) =>
            {
                acc.Add(network.Name);
                return acc;
            });
        }
        return null;
    }

    private async Task<IEnumerable<string>?> GetVolumes(string platformAddress, PlatformConnectorType connectorType, CancellationToken cancellationToken)
    {
        var args = new ListdDockerVolumesCommand
            (
                PlatformAddress: platformAddress, null, null, null
            );

        var volumeResult = await volumeConnectorFactory.GetConnector(connectorType).ListVolumesAsync(args, cancellationToken);
        if (volumeResult.IsSuccess(out var volumes))
        {
            return volumes.Aggregate(new List<string>(), (acc, volume) =>
            {
                acc.Add(volume.Id);
                return acc;
            });
        }
        return null;
    }
}
