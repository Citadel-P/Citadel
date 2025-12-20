using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Images;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.Images.Queries;

public sealed record GetExposedPorts(Guid PlatformId, Guid ImageId) : IQuery<Result<ExposedPortsResult>>
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

internal sealed class GetRunImageInfoHandler(IPlatformContainerCache platformContainerCache, IUnitOfWork unitOfWork, IConnectorFactory<IImageConnector> imgConnectorFactory) 
    : IQueryHandler<GetExposedPorts, Result<ExposedPortsResult>>
{
    public async ValueTask<Result<ExposedPortsResult>> Handle(GetExposedPorts query, CancellationToken cancellationToken)
    {
        if (!platformContainerCache.TryGetCacheEntry(query.PlatformId, out var platform, out var error))
        {
            return Result.Failure<ExposedPortsResult>(error);
        }

        var image = await unitOfWork.Images.GetByIdAsync(query.ImageId, platform.Id, cancellationToken);
        if (image is null) 
        {
            return Result.Failure<ExposedPortsResult>(new NotFoundError("Image not found."));
        }
        var args = new RunImageInfoCommand
        (
            PlatformAddress: platform.Address,
            ImageId: image.DockerImageId
        );

        var portsResult = await imgConnectorFactory.GetConnector(platform.ConnectorType).GetExposedPortsAsync(args, cancellationToken: cancellationToken);
        if (portsResult.IsFailure(out var imgError, out var result))
        {
            return Result.Failure<ExposedPortsResult>(imgError);
        }
        
        return new ExposedPortsResult
        (
            Ports: result.Ports
        );
    }
}
