using Application.Features.ResourceBindings.Models;
using Domain;
using Domain.Contracts.Interfaces;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.ResourceBindings.Commands;

public sealed record UpdateResourceBindingInputModel(
    Guid Id,
    string Name,
    ResourceBindingKind Kind,
    string? Value,
    Guid? SecretId,
    SecretDeliveryMode? SecretDeliveryMode = null,
    string? TargetPath = null)
{
    internal ResourceBindingInput ToResourceBindingInput()
        => new(Name, Kind, Value, SecretId, SecretDeliveryMode, TargetPath);
}

[RequirePermission(ResourceType.Binding, PermissionLevel.Write)]
public sealed record UpdateGlobalResourceBinding(UpdateResourceBindingInputModel Entry) : ICommand<Result<ResourceBindingsResult>>
{
    internal sealed class Validator : AbstractValidator<UpdateGlobalResourceBinding>
    {
        public Validator()
        {
            RuleFor(x => x.Entry)
                .NotNull()
                .SetValidator(new UpdateResourceBindingInputValidator(allowMountedFile: false)!);
        }
    }
}

[RequirePermission(ResourceType.Stack, PermissionLevel.Write, SpecificPermission.ResourceBindings, ResourceIdProperty = nameof(UpdateStackResourceBinding.ResourceId))]
public sealed record UpdateStackResourceBinding(Guid ResourceId, UpdateResourceBindingInputModel Entry) : ICommand<Result<ResourceBindingsResult>>
{
    internal sealed class Validator : AbstractValidator<UpdateStackResourceBinding>
    {
        public Validator()
        {
            RuleFor(x => x.Entry)
                .NotNull()
                .SetValidator(new UpdateResourceBindingInputValidator(allowMountedFile: true)!);
        }
    }
}

[RequirePermission(ResourceType.Deployment, PermissionLevel.Write, SpecificPermission.ResourceBindings, ResourceIdProperty = nameof(UpdateDeploymentResourceBinding.ResourceId))]
public sealed record UpdateDeploymentResourceBinding(Guid ResourceId, UpdateResourceBindingInputModel Entry) : ICommand<Result<ResourceBindingsResult>>
{
    internal sealed class Validator : AbstractValidator<UpdateDeploymentResourceBinding>
    {
        public Validator()
        {
            RuleFor(x => x.Entry)
                .NotNull()
                .SetValidator(new UpdateResourceBindingInputValidator(allowMountedFile: false)!);
        }
    }
}

internal sealed class UpdateResourceBindingInputValidator : AbstractValidator<UpdateResourceBindingInputModel>
{
    public UpdateResourceBindingInputValidator(bool allowMountedFile)
    {
        RuleFor(x => x.Id).NotEmpty();
        RuleFor(x => x.ToResourceBindingInput())
            .SetValidator(new ResourceBindingInputValidator(allowMountedFile));
    }
}

internal sealed class UpdateGlobalResourceBindingHandler(IUnitOfWork unitOfWork)
    : ICommandHandler<UpdateGlobalResourceBinding, Result<ResourceBindingsResult>>
{
    public async ValueTask<Result<ResourceBindingsResult>> Handle(UpdateGlobalResourceBinding command, CancellationToken cancellationToken)
        => await ResourceBindingsFeatureHelpers.UpdateEntryAsync(
            unitOfWork,
            ResourceBindingScope.Global,
            null,
            command.Entry,
            cancellationToken);
}

internal sealed class UpdateStackResourceBindingHandler(IUnitOfWork unitOfWork)
    : ICommandHandler<UpdateStackResourceBinding, Result<ResourceBindingsResult>>
{
    public async ValueTask<Result<ResourceBindingsResult>> Handle(UpdateStackResourceBinding command, CancellationToken cancellationToken)
    {
        var stack = await unitOfWork.Stacks.GetAsync(command.ResourceId, cancellationToken);
        if (stack is null)
            return Result.Failure<ResourceBindingsResult>(new NotFoundError($"Stack with ID {command.ResourceId} does not exist."));

        return await ResourceBindingsFeatureHelpers.UpdateEntryAsync(
            unitOfWork,
            ResourceBindingScope.Stack,
            command.ResourceId,
            command.Entry,
            cancellationToken);
    }
}

internal sealed class UpdateDeploymentResourceBindingHandler(IUnitOfWork unitOfWork)
    : ICommandHandler<UpdateDeploymentResourceBinding, Result<ResourceBindingsResult>>
{
    public async ValueTask<Result<ResourceBindingsResult>> Handle(UpdateDeploymentResourceBinding command, CancellationToken cancellationToken)
    {
        var deployment = await unitOfWork.Deployments.GetAsync(command.ResourceId, cancellationToken);
        if (deployment is null)
            return Result.Failure<ResourceBindingsResult>(new NotFoundError($"Deployment with ID {command.ResourceId} does not exist."));

        return await ResourceBindingsFeatureHelpers.UpdateEntryAsync(
            unitOfWork,
            ResourceBindingScope.Deployment,
            command.ResourceId,
            command.Entry,
            cancellationToken);
    }
}
