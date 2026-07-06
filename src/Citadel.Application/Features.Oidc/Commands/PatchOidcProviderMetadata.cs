using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Oidc;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.Oidc.Commands;

[RequirePermission(ResourceType.Binding, PermissionLevel.Write)]
public sealed record PatchOidcProviderMetadata(Guid Id, string? Description) : ICommand<Result<OidcProvider>>
{
    internal sealed class Validator : AbstractValidator<PatchOidcProviderMetadata>
    {
        public Validator()
        {
            RuleFor(x => x.Id).NotEmpty();
            RuleFor(x => x.Description).MaximumLength(600).When(x => x.Description is not null);
        }
    }
}

internal sealed class PatchOidcProviderMetadataHandler(IUnitOfWork unitOfWork)
    : ICommandHandler<PatchOidcProviderMetadata, Result<OidcProvider>>
{
    public async ValueTask<Result<OidcProvider>> Handle(PatchOidcProviderMetadata command, CancellationToken cancellationToken)
    {
        var existing = await unitOfWork.OidcProviders.GetAsync(command.Id, cancellationToken);
        if (existing is null)
            return Result.Failure<OidcProvider>(new NotFoundError("OIDC provider not found."));

        existing.UpdateDescription(command.Description);

        try
        {
            existing.Validate();
        }
        catch (ArgumentException ex)
        {
            return Result.Failure<OidcProvider>(new BadRequestError(ex.Message));
        }

        await unitOfWork.OidcProviders.UpdateAsync(existing, cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);

        return Result.Success(existing);
    }
}
