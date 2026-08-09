using Application.Services;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Swarm;
using Domain.Entities.Stacks;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.Abstraction;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
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
    StackSpec Spec,
    StackImportKind? ImportKind = null)
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
    IAdoptionFingerprintService fingerprintService,
    IConnectorFactory<ISwarmConnector> swarmConnectorFactory)
    : IQueryHandler<ValidateComposeProjectImportDraft, Result<ComposeProjectImportValidation>>
{
    public async ValueTask<Result<ComposeProjectImportValidation>> Handle(
        ValidateComposeProjectImportDraft query,
        CancellationToken cancellationToken)
    {
        var platform = await unitOfWork.Platforms.GetByIdAsync(query.PlatformId, cancellationToken);
        var importKind = query.ImportKind
            ?? (platform?.PlatformDescriptor.Type == PlatformType.DockerSwarm
                ? StackImportKind.SwarmStack
                : StackImportKind.ComposeProject);
        if (importKind == StackImportKind.SwarmStack)
        {
            if (platform?.PlatformDescriptor.Type != PlatformType.DockerSwarm)
            {
                return Result.Failure<ComposeProjectImportValidation>(
                    new BadRequestError("Docker Stack import requires a Docker Swarm platform."));
            }
            var swarmContextResult = await SwarmStackImportDraftFactory.LoadContextAsync(
                query.PlatformId,
                query.ProjectName,
                unitOfWork,
                swarmConnectorFactory,
                permissionService,
                userContext,
                cancellationToken);
            if (!swarmContextResult.IsSuccess(out var swarmContext))
                return Result.Failure<ComposeProjectImportValidation>(swarmContextResult.Errors);

            var swarmSourceResult = await ComposeProjectImportDraftFactory.AnalyzeSourceAsync(
                swarmContext.Platform,
                swarmContext.Namespace,
                query.Name,
                query.StackSource,
                query.Spec,
                unitOfWork,
                gitStackMaterializer,
                permissionService,
                userContext,
                cancellationToken);
            if (!swarmSourceResult.IsSuccess(out var swarmSource))
                return Result.Failure<ComposeProjectImportValidation>(swarmSourceResult.Errors);

            return SwarmStackImportDraftFactory.CreateValidation(swarmContext, swarmSource, fingerprintService);
        }

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
