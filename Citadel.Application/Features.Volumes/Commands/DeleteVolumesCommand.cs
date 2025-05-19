using Agent.Server.Volumes;
using FluentValidation;
using Grpc.Core;
using Hosting.Common.ErrorTypes;
using Infrastructure.EntityFramework;
using Infrastructure.Services.Abstractions;
using LightResults;
using Mediator;
using Microsoft.EntityFrameworkCore;

namespace Application.Features.Volumes.Commands;

public sealed record DeleteVolumesCommand(Guid PlatformId, string[] Names, bool? Force = false) : ICommand<Result>
{
    internal class Validator : AbstractValidator<DeleteVolumesCommand>
    {
        public Validator()
        {
            RuleFor(s => s.PlatformId).NotEmpty().NotNull();
            RuleForEach(s => s.Names).NotNull().NotEmpty();
        }
    }
}

internal class DeleteVolumesCommandHandler(ApplicationDbContext dbContext, IGrpcClientFactory clientFactory) : ICommandHandler<DeleteVolumesCommand, Result>
{
    public async ValueTask<Result> Handle(DeleteVolumesCommand command, CancellationToken cancellationToken)
    {
        var address = await dbContext.Platforms.Where(s => s.Id == command.PlatformId).Select(s => s.Address).FirstOrDefaultAsync(cancellationToken);
        if (address == null)
        {
            return Result.Failure(new NotFoundError("The provided platform Id doesn't exist"));
        }

        try
        {
            var client = clientFactory.GetVolumeClient(address);
            await client.RemoveAsync(new RemoveVolumeMessage { Names = { command.Names }, Force = command.Force ?? false}, cancellationToken: cancellationToken);
            return Result.Success();
        }
        catch (RpcException ex)
        {
            return Result.Failure(new ClientRpcException($"An error occurred while sending the request, {ex.Message}", ex.StatusCode));
        }
    }
}