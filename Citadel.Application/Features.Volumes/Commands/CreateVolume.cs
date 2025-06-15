using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Volumes;
using FluentValidation;
using Grpc.Core;
using Hosting.Common.ErrorTypes;
using Infrastructure.EntityFramework;
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
    Dictionary<string, string>? Options = null) : ICommand<Result<DockerVolume>>
{
    internal class Validator : AbstractValidator<CreateVolume>
    {
        public Validator()
        {
            RuleFor(s => s.Name).ValidNameIdentifier();
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

internal sealed class CreateVolumeHandler(IConnectorFactory<IVolumeConnector> connectorFactory, ApplicationDbContext dbContext) 
    : ICommandHandler<CreateVolume, Result<DockerVolume>>
{
    
    public async ValueTask<Result<DockerVolume>> Handle(CreateVolume command, CancellationToken cancellationToken)
    {
        try
        {
            var platform = await dbContext.Platforms.Where(s => s.Id == command.PlatformId)
                .Select(s => new { s.Address, s.ConnectorType }).FirstOrDefaultAsync(cancellationToken);
            if (platform == null)
            {
                return Result.Failure<DockerVolume>(new NotFoundError("The provided platform Id doesn't exist"));
            }

            var request = new CreateVolumeCommand
            (
                PlatformAddress: platform.Address,
                Name: command.Name,
                Driver: command.Driver,
                Labels: command.Labels,
                Options: command.Options
            );

            var volumeConnector = connectorFactory.GetConnector(platform.ConnectorType);
            return await volumeConnector.CreateVolumeAsync(request, cancellationToken: cancellationToken);
        }
        catch (RpcException ex)
        {
            return Result.Failure<DockerVolume>(new ClientRpcException($"An error occurred while sending the request, {ex.Message}", ex.StatusCode));
        }
    }
}
