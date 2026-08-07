using Application.Features.ResourceBindings.Commands;
using Application.Features.ResourceBindings.Models;
using Application.Features.SwarmServices;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.ResourceBindings;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.Abstraction;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.ResourceBindings.Queries
{

[RequirePermission(ResourceType.SwarmService, PermissionLevel.Read, SpecificPermission.ResourceBindings, ResourceIdProperty = nameof(Id))]
public sealed record GetSwarmServiceResourceBindings(Guid Id) : IQuery<Result<ResourceBindingsResult>>;

[RequirePermission(ResourceType.SwarmService, PermissionLevel.Read, SpecificPermission.ResourceBindings, ResourceIdProperty = nameof(Id))]
public sealed record GetSwarmServiceSecretDefinitions(Guid Id) : IQuery<Result<IReadOnlyList<SecretDefinition>>>;

internal sealed class GetSwarmServiceResourceBindingsHandler(IUnitOfWork unitOfWork, IUserContextAccessor userContext)
    : IQueryHandler<GetSwarmServiceResourceBindings, Result<ResourceBindingsResult>>
{
    public async ValueTask<Result<ResourceBindingsResult>> Handle(GetSwarmServiceResourceBindings query, CancellationToken cancellationToken) =>
        await ResourceBindingsFeatureHelpers.GetResourceEntriesAsync(
            unitOfWork,
            ResourceBindingScope.SwarmService,
            query.Id,
            async () => await SwarmServiceValidation.ExistsAndCanAccessAsync(
                query.Id,
                unitOfWork,
                userContext,
                cancellationToken),
            cancellationToken);
}

internal sealed class GetSwarmServiceSecretDefinitionsHandler(IUnitOfWork unitOfWork, IUserContextAccessor userContext)
    : IQueryHandler<GetSwarmServiceSecretDefinitions, Result<IReadOnlyList<SecretDefinition>>>
{
    public async ValueTask<Result<IReadOnlyList<SecretDefinition>>> Handle(GetSwarmServiceSecretDefinitions query, CancellationToken cancellationToken)
    {
        if (!await SwarmServiceValidation.ExistsAndCanAccessAsync(
                query.Id,
                unitOfWork,
                userContext,
                cancellationToken))
            return Result.Failure<IReadOnlyList<SecretDefinition>>(new NotFoundError($"Swarm Service with ID {query.Id} does not exist."));
        var values = await unitOfWork.SecretDefinitions.GetBoundAsync(ResourceBindingScope.SwarmService, query.Id, cancellationToken);
        return Result.Success<IReadOnlyList<SecretDefinition>>([.. values]);
    }
}

}

namespace Application.Features.ResourceBindings.Commands
{

[RequirePermission(ResourceType.SwarmService, PermissionLevel.Write, SpecificPermission.ResourceBindings, ResourceIdProperty = nameof(Id))]
public sealed record CreateSwarmServiceResourceBinding(Guid Id, ResourceBindingInput Entry) : ICommand<Result<ResourceBindingsResult>>
{
    internal sealed class Validator : AbstractValidator<CreateSwarmServiceResourceBinding>
    {
        public Validator() => RuleFor(command => command.Entry)
            .NotNull()
            .SetValidator(new ResourceBindingInputValidator(allowMountedFile: false)!);
    }
}

[RequirePermission(ResourceType.SwarmService, PermissionLevel.Write, SpecificPermission.ResourceBindings, ResourceIdProperty = nameof(Id))]
public sealed record ReplaceSwarmServiceResourceBindings(Guid Id, IReadOnlyList<ResourceBindingInput> Entries) : ICommand<Result<ResourceBindingsResult>>
{
    internal sealed class Validator : AbstractValidator<ReplaceSwarmServiceResourceBindings>
    {
        public Validator()
        {
            RuleFor(command => command.Entries).NotNull();
            RuleForEach(command => command.Entries).SetValidator(new ResourceBindingInputValidator(allowMountedFile: false));
        }
    }
}

[RequirePermission(ResourceType.SwarmService, PermissionLevel.Write, SpecificPermission.ResourceBindings, ResourceIdProperty = nameof(ResourceId))]
public sealed record UpdateSwarmServiceResourceBinding(Guid ResourceId, UpdateResourceBindingInputModel Entry) : ICommand<Result<ResourceBindingsResult>>;

[RequirePermission(ResourceType.SwarmService, PermissionLevel.Write, SpecificPermission.ResourceBindings, ResourceIdProperty = nameof(ResourceId))]
public sealed record DeleteSwarmServiceResourceBinding(Guid ResourceId, Guid Id) : ICommand<Result<ResourceBindingsResult>>;

internal sealed class SwarmServiceResourceBindingHandler(IUnitOfWork unitOfWork, IUserContextAccessor userContext)
    : ICommandHandler<CreateSwarmServiceResourceBinding, Result<ResourceBindingsResult>>,
      ICommandHandler<ReplaceSwarmServiceResourceBindings, Result<ResourceBindingsResult>>,
      ICommandHandler<UpdateSwarmServiceResourceBinding, Result<ResourceBindingsResult>>,
      ICommandHandler<DeleteSwarmServiceResourceBinding, Result<ResourceBindingsResult>>
{
    public async ValueTask<Result<ResourceBindingsResult>> Handle(CreateSwarmServiceResourceBinding command, CancellationToken cancellationToken)
    {
        var exists = await ExistsAsync(command.Id, cancellationToken);
        return exists.IsFailure(out var error)
            ? Result.Failure<ResourceBindingsResult>(error)
            : await ResourceBindingsFeatureHelpers.CreateEntryAsync(
                unitOfWork, ResourceBindingScope.SwarmService, command.Id, command.Entry, cancellationToken);
    }

    public async ValueTask<Result<ResourceBindingsResult>> Handle(ReplaceSwarmServiceResourceBindings command, CancellationToken cancellationToken)
    {
        var exists = await ExistsAsync(command.Id, cancellationToken);
        return exists.IsFailure(out var error)
            ? Result.Failure<ResourceBindingsResult>(error)
            : await ResourceBindingsFeatureHelpers.ReplaceEntriesAsync(
                unitOfWork, ResourceBindingScope.SwarmService, command.Id, command.Entries, cancellationToken);
    }

    public async ValueTask<Result<ResourceBindingsResult>> Handle(UpdateSwarmServiceResourceBinding command, CancellationToken cancellationToken)
    {
        var exists = await ExistsAsync(command.ResourceId, cancellationToken);
        return exists.IsFailure(out var error)
            ? Result.Failure<ResourceBindingsResult>(error)
            : await ResourceBindingsFeatureHelpers.UpdateEntryAsync(
                unitOfWork, ResourceBindingScope.SwarmService, command.ResourceId, command.Entry, cancellationToken);
    }

    public async ValueTask<Result<ResourceBindingsResult>> Handle(DeleteSwarmServiceResourceBinding command, CancellationToken cancellationToken)
    {
        var exists = await ExistsAsync(command.ResourceId, cancellationToken);
        return exists.IsFailure(out var error)
            ? Result.Failure<ResourceBindingsResult>(error)
            : await ResourceBindingsFeatureHelpers.DeleteEntryAsync(
                unitOfWork, ResourceBindingScope.SwarmService, command.ResourceId, command.Id, cancellationToken);
    }

    private async Task<Result> ExistsAsync(Guid id, CancellationToken cancellationToken) =>
        !await SwarmServiceValidation.ExistsAndCanAccessAsync(id, unitOfWork, userContext, cancellationToken)
            ? Result.Failure(new NotFoundError($"Swarm Service with ID {id} does not exist."))
            : Result.Success();
}

}
