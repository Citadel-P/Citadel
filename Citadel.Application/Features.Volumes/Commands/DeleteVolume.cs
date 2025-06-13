using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Volumes;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.ErrorTypes;
using Infrastructure.EntityFramework;
using LightResults;
using Mediator;
using Microsoft.EntityFrameworkCore;

namespace Application.Features.Volumes.Commands;

public sealed record DeleteVolume(Guid PlatformId, string[] Names, bool? Force = false) : ICommand<Result>
{
    internal class Validator : AbstractValidator<DeleteVolume>
    {
        public Validator()
        {
            RuleFor(s => s.PlatformId).NotEmpty().NotNull();
            RuleForEach(s => s.Names).ValidNameIdentifier();
        }
    }
}

internal class DeleteVolumeHandler(IVolumeConnector volumeConnector, ApplicationDbContext dbContext) : ICommandHandler<DeleteVolume, Result>
{
    public async ValueTask<Result> Handle(DeleteVolume command, CancellationToken cancellationToken)
    {
        var address = await dbContext.Platforms.Where(s => s.Id == command.PlatformId).Select(s => s.Address).FirstOrDefaultAsync(cancellationToken);
        if (address == null)
        {
            return Result.Failure(new NotFoundError("The provided platform Id doesn't exist"));
        }

        var args = new DeleteVolumeCommand
        (
            PlatformAddress: address,
            Names: command.Names,
            Force: command.Force ?? false
        );
        return await volumeConnector.DeleteVolumeAsync(args, cancellationToken);
    }
}