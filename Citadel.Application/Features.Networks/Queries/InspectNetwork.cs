using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Networks;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.ErrorTypes;
using Infrastructure.EntityFramework;
using LightResults;
using Mediator;
using Microsoft.EntityFrameworkCore;

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

internal sealed class InspectNetworkHandler(IConnectorFactory<INetworkConnector> connectorFactory, ApplicationDbContext dbContext) : IQueryHandler<InspectNetwork, Result<DockerNetworkDetails>>
{
    public async ValueTask<Result<DockerNetworkDetails>> Handle(InspectNetwork query, CancellationToken cancellationToken)
    {
        var platform = await dbContext.Platforms.Where(s => s.Id == query.PlatformId)
            .Select(s => new { s.Address, s.ConnectorType }).FirstOrDefaultAsync(cancellationToken);
        if (platform == null)
        {
            return Result.Failure<DockerNetworkDetails>(new NotFoundError("The provided platform Id doesn't exist"));
        }

        var args = new InspectNetworkCommand
        (
            NetworkId: query.NetworkId, 
            PlatformAddress: platform.Address
        );

        var networkConnector = connectorFactory.GetConnector(platform.ConnectorType);
        return await networkConnector.InspectNetworkAsync(args, cancellationToken);
    }
}
