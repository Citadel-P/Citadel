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

public sealed record class DeleteNetworksCommand(Guid PlatformId, string[] Ids) : ICommand<Result>
{
    internal class Validator : AbstractValidator<DeleteNetworksCommand>
    {
        public Validator()
        {
            RuleFor(s => s.PlatformId).NotEmpty().NotNull();
            RuleForEach(s => s.Ids).ValidHashId();
        }
    }
}

internal class DeleteNetworksCommandHandler(INetworkConnector networkConnector, ApplicationDbContext dbContext) : ICommandHandler<DeleteNetworksCommand, Result>
{
    public async ValueTask<Result> Handle(DeleteNetworksCommand command, CancellationToken cancellationToken)
    {
        var address = await dbContext.Platforms.Where(s => s.Id == command.PlatformId).Select(s => s.Address).FirstOrDefaultAsync(cancellationToken);
        if (address == null)
        {
            return Result.Failure(new NotFoundError("The provided platform Id doesn't exist"));
        }

        var args = new DeleteNetworkCommand
        (
            PlatformAddress: address,
            Ids: command.Ids
        );
        return await networkConnector.DeleteNetworkAsync(args, cancellationToken);
    }
}
