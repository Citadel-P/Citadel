using Agent.Server.Volumes;
using FluentValidation;
using Grpc.Core;
using Hosting.Common.ErrorTypes;
using Infrastructure.EntityFramework;
using Infrastructure.Services.Abstractions;
using LightResults;
using Mediator;
using Microsoft.EntityFrameworkCore;
using static Hosting.Common.Validators;

namespace Application.Features.Volumes.Commands;

public sealed record CreateVolume(
    Guid PlatformId,
    string Name,
    string Driver,
    Dictionary<string, string>? Labels = null,
    Dictionary<string, string>? Options = null) : ICommand<Result<VolumeReply>>
{
    internal class Validator : AbstractValidator<CreateVolume>
    {
        public Validator()
        {
            RuleFor(s => s.Name).NotNull().NotEmpty();
            When(s => s.Driver is not null, () =>
            {
                RuleFor(s => s.Driver)
                .Must(static driver => driver == "local")
                .WithMessage("Driver must be 'local'.");
            });
            When(s => s.Labels is not null, () =>
            {
                RuleForEach(s => s.Labels).SetValidator(new KeyPairValidator());
            });
            When(s => s.Options is not null, () =>
            {
                RuleForEach(s => s.Options).SetValidator(new KeyPairValidator());
            });
        }
    }
}

internal sealed class CreateVolumeHandler(IGrpcClientFactory clientFactory,ApplicationDbContext dbContext) 
    : ICommandHandler<CreateVolume, Result<VolumeReply>>
{
    
    public async ValueTask<Result<VolumeReply>> Handle(CreateVolume command, CancellationToken cancellationToken)
    {
        try
        {
            var address = await dbContext.Platforms.Where(s => s.Id == command.PlatformId).Select(s => s.Address).FirstOrDefaultAsync(cancellationToken);
            if (address == null)
            {
                return Result.Failure<VolumeReply>(new NotFoundError("The provided platform Id doesn't exist"));
            }

            var client = clientFactory.GetVolumeClient(address);
            var request = new CreateVolumeMessage
            {
                Name = command.Name,
                Driver = command.Driver,
                Labels = { command.Labels ?? [] },
                Options = { command.Options ?? [] }
            };

            return await client.CreateAsync(request, cancellationToken: cancellationToken);
        }
        catch (RpcException ex)
        {
            return Result.Failure<VolumeReply>(new ClientRpcException($"An error occurred while sending the request, {ex.Message}", ex.StatusCode));
        }
    }
}
