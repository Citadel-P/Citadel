using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Networks;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.Networks.Queries;

public sealed record InspectNetwork(Guid PlatformId, string NetworkId): IQuery<Result<DockerNetworkDetails>>
{
    internal class Validator : AbstractValidator<InspectNetwork>
    {
        public Validator()
        {
            RuleFor(s => s.PlatformId).NotEmpty().NotNull();
            RuleFor(s => s.NetworkId).ValidHashId();
        }
    }
}

internal sealed class InspectNetworkHandler(IUnitOfWork unitOfWork, IConnectorFactory<INetworkConnector> connectorFactory) 
    : IQueryHandler<InspectNetwork, Result<DockerNetworkDetails>>
{
    public async ValueTask<Result<DockerNetworkDetails>> Handle(InspectNetwork query, CancellationToken cancellationToken)
    {
        var platform = await unitOfWork.Platforms.GetPlatformInfoAsync(query.PlatformId, cancellationToken);
        if (platform is null)
        {
            return Result.Failure<DockerNetworkDetails>(new NotFoundError("The provided platform Id doesn't exist"));
        }

        var args = new InspectNetworkCommand
        (
            NetworkId: query.NetworkId, 
            PlatformAddress: platform.Value.Address
        );

        var networkConnector = connectorFactory.GetConnector(platform.Value.ConnectorType);
        return await networkConnector.InspectNetworkAsync(args, cancellationToken);
    }
}
