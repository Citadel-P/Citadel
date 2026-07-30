using Application.Services;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Stacks;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.Abstraction;
using Hosting.Common.Attributes;
using Hosting.Common.Extensions;
using Hosting.Common.Pipelines.Interfaces;
using LightResults;
using Mediator;

namespace Application.Features.Stacks.Queries;

[RequirePermission(ResourceType.Stack, PermissionLevel.Write)]
public sealed record ValidateComposeProjectImportDraft(
    Guid PlatformId,
    string ProjectName,
    string Name,
    StackSource StackSource,
    StackSpec Spec)
    : IQuery<Result<ComposeProjectImportValidation>>
{
    internal sealed class Validator : AbstractValidator<ValidateComposeProjectImportDraft>
    {
        public Validator()
        {
            RuleFor(x => x.ProjectName).NotEmpty().MaximumLength(128);
            RuleFor(x => x.Name).NotEmpty().ValidNameIdentifier();
            RuleFor(x => x.Spec).NotNull();
        }
    }
}

internal sealed class ValidateComposeProjectImportDraftHandler(
    IUnitOfWork unitOfWork,
    IConnectorFactory<IContainerConnector> connectorFactory,
    IContainerAuthorizationService containerAuthorizationService,
    IGitStackMaterializer gitStackMaterializer,
    IPermissionService permissionService,
    IUserContextAccessor userContext,
    IAdoptionFingerprintService fingerprintService)
    : IQueryHandler<ValidateComposeProjectImportDraft, Result<ComposeProjectImportValidation>>
{
    public async ValueTask<Result<ComposeProjectImportValidation>> Handle(
        ValidateComposeProjectImportDraft query,
        CancellationToken cancellationToken)
    {
        var contextResult = await ComposeProjectImportDraftFactory.LoadContextAsync(
            query.PlatformId,
            query.ProjectName,
            unitOfWork,
            connectorFactory,
            containerAuthorizationService,
            cancellationToken);
        if (!contextResult.IsSuccess(out var context))
            return Result.Failure<ComposeProjectImportValidation>(contextResult.Errors);

        var sourceResult = await ComposeProjectImportDraftFactory.AnalyzeSourceAsync(
            context,
            query.Name,
            query.StackSource,
            query.Spec,
            unitOfWork,
            gitStackMaterializer,
            permissionService,
            userContext,
            cancellationToken);
        if (!sourceResult.IsSuccess(out var source))
            return Result.Failure<ComposeProjectImportValidation>(sourceResult.Errors);

        return ComposeProjectImportDraftFactory.CreateValidation(context, source, fingerprintService);
    }
}
