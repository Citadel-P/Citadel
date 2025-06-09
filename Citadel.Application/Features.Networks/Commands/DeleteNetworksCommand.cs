using Citadel.Agent.Networks.V1;
using FluentValidation;
using Grpc.Core;
using Hosting.Common;
using Hosting.Common.ErrorTypes;
using Infrastructure.EntityFramework;
using Infrastructure.Services.Abstractions;
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

internal class DeleteNetworksCommandHandler(ApplicationDbContext dbContext, IGrpcClientFactory clientFactory) : ICommandHandler<DeleteNetworksCommand, Result>
{
    public async ValueTask<Result> Handle(DeleteNetworksCommand command, CancellationToken cancellationToken)
    {
        var address = await dbContext.Platforms.Where(s => s.Id == command.PlatformId).Select(s => s.Address).FirstOrDefaultAsync(cancellationToken);
        if (address == null)
        {
            return Result.Failure(new NotFoundError("The provided platform Id doesn't exist"));
        }

        try
        {
            var client = clientFactory.GetNetworkClient(address);
            await client.DeleteAsync(new DeleteNetworkMessage { Ids = { command.Ids } }, cancellationToken: cancellationToken);
            return Result.Success();
        }
        catch (RpcException ex)
        {
            return Result.Failure(new ClientRpcException($"An error occurred while sending the request, {ex.Message}", ex.StatusCode));
        }
    }
}
