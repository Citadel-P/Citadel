using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Images;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.Images.Queries;

public sealed record GetRunImageInfo(Guid PlatformId, string ImageId) : IQuery<Result<RunImageInfoResult>>
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

internal sealed class GetRunImageInfoHandler(IUnitOfWork unitOfWork, IConnectorFactory<IImageConnector> imgConnectorFactory) 
    : IQueryHandler<GetRunImageInfo, Result<RunImageInfoResult>>
{
    public async ValueTask<Result<RunImageInfoResult>> Handle(GetRunImageInfo query, CancellationToken cancellationToken)
    {
        var platform = await unitOfWork.Platforms.GetByIdAsync(query.PlatformId, cancellationToken);
        if (platform is null)
        {
            return Result.Failure<RunImageInfoResult>(new NotFoundError("Platform not found."));
        }
        var args = new RunImageInfoCommand
        (
            PlatformAddress: platform.Address,
            ImageId: query.ImageId
        );

        var infoResult = await imgConnectorFactory.GetConnector(platform.ConnectorType).GetRunImageInfoAsync(args, cancellationToken: cancellationToken);
        if (infoResult.IsFailure(out var error, out var runInfo))
        {
            return Result.Failure<RunImageInfoResult>(error);
        }
        
        return new RunImageInfoResult
        (
            Volumes: runInfo.Volumes,
            Networks: runInfo.Networks,
            ExposedPorts: runInfo.ExposedPorts,
            MemTotal: platform.MemTotal,
            CpuCount: platform.CpuCount
        );
    }
}
