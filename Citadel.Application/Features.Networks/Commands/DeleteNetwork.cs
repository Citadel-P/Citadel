using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Networks;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.ErrorTypes;
using Infrastructure.EntityFramework;
using LightResults;
using Mediator;
using Microsoft.EntityFrameworkCore;

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

internal class DeleteNetworksHandler(IConnectorFactory<INetworkConnector> connectorFactory, ApplicationDbContext dbContext) : ICommandHandler<DeleteNetwork, Result>
{
    public async ValueTask<Result> Handle(DeleteNetwork command, CancellationToken cancellationToken)
    {
        var platform = await dbContext.Platforms.Where(s => s.Id == command.PlatformId)
            .Select(s => new { s.Address, s.ConnectorType }).FirstOrDefaultAsync(cancellationToken);
        if (platform == null)
        {
            return Result.Failure(new NotFoundError("The provided platform Id doesn't exist"));
        }

        var args = new DeleteNetworkCommand
        (
            PlatformAddress: platform.Address,
            Ids: command.Ids
        );

        var networkConnector = connectorFactory.GetConnector(platform.ConnectorType);
        return await networkConnector.DeleteNetworkAsync(args, cancellationToken);
    }
}
