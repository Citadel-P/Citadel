using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Networks;
using FluentValidation;
using Hosting.Common;
using LightResults;
using Mediator;

namespace Application.Features.Networks.Commands;

public sealed record class DeleteNetworks(Guid PlatformId, string[] Ids) : ICommand<Result>
{
    internal class Validator : AbstractValidator<DeleteNetworks>
    {
        public Validator()
        {
            RuleFor(s => s.PlatformId).NotEmpty().NotNull();
            RuleForEach(s => s.Ids).ValidHashId();
        }
    }
}

internal class DeleteNetworksHandler(IPlatformContainerCache platformContainerCache, IConnectorFactory<INetworkConnector> connectorFactory) : ICommandHandler<DeleteNetworks, Result>
{
    public async ValueTask<Result> Handle(DeleteNetworks command, CancellationToken cancellationToken)
    {
        if (!platformContainerCache.TryGetCacheEntry(command.PlatformId, out var platform, out var error))
        {
            return Result.Failure(error);
        }

        var args = new DeleteDockerNetworkCommand
        (
            PlatformAddress: platform.Address,
            Ids: command.Ids
        );

        var networkConnector = connectorFactory.GetConnector(platform.ConnectorType);
        return await networkConnector.DeleteNetworkAsync(args, cancellationToken);
    }
}
