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

public sealed record InspectVolume (Guid PlatformId, string Name) : IQuery<Result<VolumeResponse>>
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

internal sealed class InspectVolumeHandler(ApplicationDbContext dbContext, IGrpcClientFactory clientFactory) : IQueryHandler<InspectVolume, Result<VolumeResponse>>
{
    public async ValueTask<Result<VolumeResponse>> Handle(InspectVolume query, CancellationToken cancellationToken)
    {
        var address = await dbContext.Platforms.Where(s => s.Id == query.PlatformId).Select(s => s.Address).FirstOrDefaultAsync(cancellationToken);
        if (address == null)
        {
            return Result.Failure<VolumeResponse>(new NotFoundError("The provided platform Id doesn't exist"));
        }
        try
        {
            var request = new InspectVolumeRequest
            {
                Name = query.Name
            };
            var client = clientFactory.GetVolumeClient(address);
            return await client.InspectAsync(request, cancellationToken: cancellationToken);
        }
        catch (RpcException ex)
        {
            return Result.Failure<VolumeResponse>(new ClientRpcException($"An error occurred while sending the request, {ex.Message}", ex.StatusCode));
        }
    }
}