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

[RequirePermission(ResourceType.Binding, PermissionLevel.Write)]
public sealed record CreateGlobalResourceBinding(ResourceBindingInput Entry) : ICommand<Result<ResourceBindingsResult>>
{
    internal sealed class Validator : AbstractValidator<CreateGlobalResourceBinding>
    {
        public Validator()
        {
            RuleFor(x => x.Entry)
                .NotNull()
                .SetValidator(new ResourceBindingInputValidator(allowMountedFile: false)!);
        }
    }
}

[RequirePermission(ResourceType.Stack, PermissionLevel.Write, SpecificPermission.ResourceBindings)]
public sealed record CreateStackResourceBinding(Guid Id, ResourceBindingInput Entry) : ICommand<Result<ResourceBindingsResult>>
{
    internal sealed class Validator : AbstractValidator<CreateStackResourceBinding>
    {
        public Validator()
        {
            RuleFor(x => x.Entry)
                .NotNull()
                .SetValidator(new ResourceBindingInputValidator(allowMountedFile: true)!);
        }
    }
}

[RequirePermission(ResourceType.Deployment, PermissionLevel.Write, SpecificPermission.ResourceBindings)]
public sealed record CreateDeploymentResourceBinding(Guid Id, ResourceBindingInput Entry) : ICommand<Result<ResourceBindingsResult>>
{
    internal sealed class Validator : AbstractValidator<CreateDeploymentResourceBinding>
    {
        public Validator()
        {
            RuleFor(x => x.Entry)
                .NotNull()
                .SetValidator(new ResourceBindingInputValidator(allowMountedFile: false)!);
        }
    }
}

internal sealed class CreateGlobalResourceBindingHandler(IUnitOfWork unitOfWork)
    : ICommandHandler<CreateGlobalResourceBinding, Result<ResourceBindingsResult>>
{
    public async ValueTask<Result<ResourceBindingsResult>> Handle(CreateGlobalResourceBinding command, CancellationToken cancellationToken)
        => await ResourceBindingsFeatureHelpers.CreateEntryAsync(
            unitOfWork,
            ResourceBindingScope.Global,
            null,
            command.Entry,
            cancellationToken);
}

internal sealed class CreateStackResourceBindingHandler(IUnitOfWork unitOfWork)
    : ICommandHandler<CreateStackResourceBinding, Result<ResourceBindingsResult>>
{
    public async ValueTask<Result<ResourceBindingsResult>> Handle(CreateStackResourceBinding command, CancellationToken cancellationToken)
    {
        var stack = await unitOfWork.Stacks.GetAsync(command.Id, cancellationToken);
        if (stack is null)
            return Result.Failure<ResourceBindingsResult>(new NotFoundError($"Stack with ID {command.Id} does not exist."));

        return await ResourceBindingsFeatureHelpers.CreateEntryAsync(
            unitOfWork,
            ResourceBindingScope.Stack,
            command.Id,
            command.Entry,
            cancellationToken);
    }
}

internal sealed class CreateDeploymentResourceBindingHandler(IUnitOfWork unitOfWork)
    : ICommandHandler<CreateDeploymentResourceBinding, Result<ResourceBindingsResult>>
{
    public async ValueTask<Result<ResourceBindingsResult>> Handle(CreateDeploymentResourceBinding command, CancellationToken cancellationToken)
    {
        var deployment = await unitOfWork.Deployments.GetAsync(command.Id, cancellationToken);
        if (deployment is null)
            return Result.Failure<ResourceBindingsResult>(new NotFoundError($"Deployment with ID {command.Id} does not exist."));

        return await ResourceBindingsFeatureHelpers.CreateEntryAsync(
            unitOfWork,
            ResourceBindingScope.Deployment,
            command.Id,
            command.Entry,
            cancellationToken);
    }
}
