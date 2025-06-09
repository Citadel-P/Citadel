using Citadel.Agent.Volumes.V1;
using FluentValidation;
using Grpc.Core;
using Hosting.Common.ErrorTypes;
using Infrastructure.EntityFramework;
using Infrastructure.Services.Abstractions;
using LightResults;
using Mediator;
using Microsoft.EntityFrameworkCore;

namespace Application.Features.Volumes.Queries;

public sealed record InspectVolume (Guid PlatformId, string Name) : IQuery<Result<VolumeReply>>
{
    internal class Validator : AbstractValidator<InspectVolume>
    {
        public Validator()
        {
            RuleFor(s => s.PlatformId).NotEmpty().NotNull();
            RuleFor(s => s.Name).NotNull();
        }
    }
}

internal sealed class InspectVolumeHandler(ApplicationDbContext dbContext, IGrpcClientFactory clientFactory) : IQueryHandler<InspectVolume, Result<VolumeReply>>
{
    public async ValueTask<Result<VolumeReply>> Handle(InspectVolume query, CancellationToken cancellationToken)
    {
        var address = await dbContext.Platforms.Where(s => s.Id == query.PlatformId).Select(s => s.Address).FirstOrDefaultAsync(cancellationToken);
        if (address == null)
        {
            return Result.Failure<VolumeReply>(new NotFoundError("The provided platform Id doesn't exist"));
        }
        try
        {
            var args = new InspectVolumeMessage
            {
                Name = query.Name
            };
            var client = clientFactory.GetVolumeClient(address);
            return await client.InspectAsync(args, cancellationToken: cancellationToken);
        }
        catch (RpcException ex)
        {
            return Result.Failure<VolumeReply>(new ClientRpcException($"An error occurred while sending the request, {ex.Message}", ex.StatusCode));
        }
    }
}