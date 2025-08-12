using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Networks;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;
using static Google.Rpc.Context.AttributeContext.Types;

namespace Application.Features.Networks.Commands;

public sealed record class DeleteNetwork(Guid PlatformId, string[] Ids) : ICommand<Result>
{
    internal class Validator : AbstractValidator<DeleteNetwork>
    {
        public Validator()
        {
            RuleFor(s => s.PlatformId).NotEmpty().NotNull();
            RuleForEach(s => s.Ids).ValidHashId();
        }
    }
}

internal class DeleteNetworksHandler(IPlatformContainerCache platformContainerCache, IConnectorFactory<INetworkConnector> connectorFactory) : ICommandHandler<DeleteNetwork, Result>
{
    public async ValueTask<Result> Handle(DeleteNetwork command, CancellationToken cancellationToken)
    {
        if (!platformContainerCache.TryGetCacheEntry(command.PlatformId, out var platform))
        {
            return Result.Failure(new NotFoundError("Platform ID not found."));
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
