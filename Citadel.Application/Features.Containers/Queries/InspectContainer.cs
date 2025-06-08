using Agent.Server.Containers;
using Citadel.Common;
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

public sealed record InspectContainer(string ContainerId) : IQuery<Result<ContainerInspectReply>>
{
    internal sealed class Validator : AbstractValidator<InspectContainer>
    {
        public Validator()
            => RuleFor(s => s.ContainerId).ValidContainerId();
    }
}

internal sealed class InspectContainerHandler(
    IGrpcClientFactory clientFactory, 
    ApplicationDbContext dbContext) : IQueryHandler<InspectContainer, Result<ContainerInspectReply>>
{
    
    public async ValueTask<Result<ContainerInspectReply>> Handle(InspectContainer query, CancellationToken cancellationToken)
    {
        var platformAddress = await dbContext.Containers.GetPlatformAddress(query.ContainerId, cancellationToken);
        if (platformAddress == null)
        {
            return Result.Failure<ContainerInspectReply>(new NotFoundError($"Platform doesn't exist for container {query.ContainerId}"));
        }

        var client = clientFactory.GetContainerClient(platformAddress);
        var request = new InspectContainerRequest() { ContainerId = query.ContainerId };
        try
        {
            return await client.InspectContainerAsync(request, cancellationToken: cancellationToken);
        }
        catch (RpcException ex) 
        {
            return Result.Failure<ContainerInspectReply>(new ClientRpcException($"An error occurred while sending the request, {ex.Message}", ex.StatusCode));
        }
    }
}