using Citadel.Agent.Common.V1;
using Citadel.Agent.Containers.V1;
using FluentValidation;
using Grpc.Core;
using Hosting.Common;
using Hosting.Common.ErrorTypes;
using Infrastructure;
using Infrastructure.EntityFramework;
using Infrastructure.Services.Abstractions;
using LightResults;
using Mediator;

namespace Application.Features.Containers.Queries;

public sealed record InspectContainer(string ContainerId) : IQuery<Result<InspectContainerResponse>>
{
    internal sealed class Validator : AbstractValidator<InspectContainer>
    {
        public Validator()
            => RuleFor(s => s.ContainerId).ValidContainerId();
    }
}

internal sealed class InspectContainerHandler(
    IGrpcClientFactory clientFactory, 
    ApplicationDbContext dbContext) : IQueryHandler<InspectContainer, Result<InspectContainerResponse>>
{
    
    public async ValueTask<Result<InspectContainerResponse>> Handle(InspectContainer query, CancellationToken cancellationToken)
    {
        var platformAddress = await dbContext.Containers.GetPlatformAddress(query.ContainerId, cancellationToken);
        if (platformAddress == null)
        {
            return Result.Failure<InspectContainerResponse>(new NotFoundError($"Platform doesn't exist for container {query.ContainerId}"));
        }

        var client = clientFactory.GetContainerClient(platformAddress);
        var request = new InspectContainerRequest() { ContainerId = query.ContainerId };
        try
        {
            return await client.InspectAsync(request, cancellationToken: cancellationToken);
        }
        catch (RpcException ex) 
        {
            return Result.Failure<InspectContainerResponse>(new ClientRpcException($"An error occurred while sending the request, {ex.Message}", ex.StatusCode));
        }
    }
}