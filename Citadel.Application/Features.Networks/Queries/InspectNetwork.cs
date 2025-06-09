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

namespace Application.Features.Networks.Queries;

public sealed record InspectNetwork(Guid PlatformId, string NetworkId): IQuery<Result<InspectNetworkResponse>>
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

internal sealed class InspectNetworkHandler(ApplicationDbContext dbContext, IGrpcClientFactory clientFactory) : IQueryHandler<InspectNetwork, Result<InspectNetworkResponse>>
{
    public async ValueTask<Result<InspectNetworkResponse>> Handle(InspectNetwork query, CancellationToken cancellationToken)
    {
        var address = await dbContext.Platforms.Where(s => s.Id == query.PlatformId).Select(s => s.Address).FirstOrDefaultAsync(cancellationToken);
        if (address == null)
        {
            return Result.Failure<InspectNetworkResponse>(new NotFoundError("The provided platform Id doesn't exist"));
        }
        try
        {
            var request = new InspectNetworkRequest
            {
                Id = query.NetworkId
            };
            var client = clientFactory.GetNetworkClient(address);
            return await client.InspectAsync(request, cancellationToken: cancellationToken);
        }
        catch (RpcException ex)
        {
            return Result.Failure<InspectNetworkResponse>(new ClientRpcException($"An error occurred while sending the request, {ex.Message}", ex.StatusCode));
        }
    }
}
