using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Activities;
using Domain.Entities.Oidc;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.Abstraction;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.Oidc.Commands;

[RequirePermission(ResourceType.Binding, PermissionLevel.Write)]
public sealed record RenameOidcProvider(Guid Id, string Name) : ICommand<Result<OidcProvider>>
{
    internal sealed class Validator : AbstractValidator<RenameOidcProvider>
    {
        public Validator()
        {
            RuleFor(x => x.Id).NotEmpty();
            RuleFor(x => x.Name).NotEmpty().MaximumLength(128);
        }
    }
}

internal sealed class RenameOidcProviderHandler(IUnitOfWork unitOfWork, IUserContextAccessor userContextAccessor)
    : ICommandHandler<RenameOidcProvider, Result<OidcProvider>>
{
    public async ValueTask<Result<OidcProvider>> Handle(RenameOidcProvider command, CancellationToken cancellationToken)
    {
        var existing = await unitOfWork.OidcProviders.GetAsync(command.Id, cancellationToken);
        if (existing is null)
            return Result.Failure<OidcProvider>(new NotFoundError("OIDC provider not found."));

        if (await unitOfWork.OidcProviders.ExistsByNameExceptAsync(command.Name, command.Id, cancellationToken))
            return Result.Failure<OidcProvider>(new ConflictError("OIDC provider name already exists."));

        var oldProvider = OidcProviderActivity.ToSnapshot(existing);
        existing.Rename(command.Name);

        try
        {
            existing.Validate();
        }
        catch (ArgumentException ex)
        {
            return Result.Failure<OidcProvider>(new BadRequestError(ex.Message));
        }

        await unitOfWork.OidcProviders.UpdateAsync(existing, cancellationToken);
        await unitOfWork.ActivityEventRepository.AddAsync(
            new ActivityEvent(
                platformId: null,
                resourceId: existing.Id,
                actorId: userContextAccessor.Current.ActorId,
                resourceName: existing.Name,
                eventType: ActivityEventType.OidcProviderUpdated,
                status: ActivityStatus.Success,
                info: new OidcProviderUpdated(oldProvider, OidcProviderActivity.ToSnapshot(existing))),
            cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);

        return Result.Success(existing);
    }
}
