using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Platforms;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.Platforms.Commands;

[RequirePermission(ResourceType.Platform, PermissionLevel.Execute)]
public sealed record PrunePlatform(Guid PlatformId, PruneResource Resource) : ICommand<Result<PrunePlatformResult>>
{
    internal class Validator : AbstractValidator<PrunePlatform>
    {
        public Validator()
        {
            RuleFor(x => x.PlatformId).NotEmpty();
            RuleFor(x => x.Resource).IsInEnum();
        }
    }
}

internal sealed class PrunePlatformHandler(
    IUnitOfWork unitOfWork,
    IConnectorFactory<IPlatformConnector> connectorFactory) : ICommandHandler<PrunePlatform, Result<PrunePlatformResult>>
{
    public async ValueTask<Result<PrunePlatformResult>> Handle(PrunePlatform command, CancellationToken cancellationToken)
    {
        var platform = await unitOfWork.Platforms.GetInfoAsync(command.PlatformId, cancellationToken);
        if (platform is null)
        {
            return Result.Failure<PrunePlatformResult>(new NotFoundError("Platform does not exist."));
        }

        var connector = connectorFactory.GetConnector(platform.ConnectorType);
        return await connector.PruneAsync(
            new PrunePlatformCommand(platform.Address, command.Resource),
            cancellationToken);
    }
}
