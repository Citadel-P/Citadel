using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Volumes;
using FluentValidation;
using Grpc.Core;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;
using static Hosting.Common.Validators;

namespace Application.Features.Volumes.Commands;

public sealed record CreateVolume(
    Guid PlatformId,
    string Name,
    string Driver,
    Dictionary<string, string>? Labels = null,
    Dictionary<string, string>? Options = null) : ICommand<Result<DockerVolumeResult>>
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

internal sealed class CreateVolumeHandler(IPlatformContainerCache platformContainerCache, IConnectorFactory<IVolumeConnector> connectorFactory) 
    : ICommandHandler<CreateVolume, Result<DockerVolumeResult>>
{
    
    public async ValueTask<Result<DockerVolumeResult>> Handle(CreateVolume command, CancellationToken cancellationToken)
    {
        try
        {
            if (!platformContainerCache.TryGetCacheEntry(command.PlatformId, out var platform))
            {
                return Result.Failure<DockerVolumeResult>(new NotFoundError("The provided platform Id doesn't exist"));
            }

            var request = new CreateDockerVolumeCommand
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
            return Result.Failure<DockerVolumeResult>(new ClientRpcException($"An error occurred while sending the request, {ex.Message}", ex.StatusCode));
        }
    }
}
