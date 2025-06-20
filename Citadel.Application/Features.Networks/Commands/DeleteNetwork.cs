using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Networks;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

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

internal class DeleteNetworksHandler(IUnitOfWork unitOfWork, IConnectorFactory<INetworkConnector> connectorFactory) : ICommandHandler<DeleteNetwork, Result>
{
    public async ValueTask<Result> Handle(DeleteNetwork command, CancellationToken cancellationToken)
    {
        var (address, connectorType) = await unitOfWork.Platforms.GetPlatformInfoAsync(command.PlatformId, cancellationToken);
        if (string.IsNullOrEmpty(address))
        {
            return Result.Failure(new NotFoundError("The provided platform Id doesn't exist"));
        }

        var args = new DeleteDockerNetworkCommand
        (
            PlatformAddress: address,
            Ids: command.Ids
        );

        var networkConnector = connectorFactory.GetConnector(connectorType);
        return await networkConnector.DeleteNetworkAsync(args, cancellationToken);
    }
}
