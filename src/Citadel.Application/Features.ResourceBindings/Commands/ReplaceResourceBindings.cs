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
public sealed record ReplaceGlobalResourceBindings(IReadOnlyList<ResourceBindingInput> Entries) : ICommand<Result<ResourceBindingsResult>>
{
    internal sealed class Validator : AbstractValidator<ReplaceGlobalResourceBindings>
    {
        public Validator()
        {
            RuleFor(x => x.Entries).NotNull();
            RuleForEach(x => x.Entries).SetValidator(new ResourceBindingInputValidator(allowMountedFile: false));
        }
    }
}

[RequirePermission(ResourceType.Stack, PermissionLevel.Write, SpecificPermission.ResourceBindings)]
public sealed record ReplaceStackResourceBindings(Guid Id, IReadOnlyList<ResourceBindingInput> Entries) : ICommand<Result<ResourceBindingsResult>>
{
    internal sealed class Validator : AbstractValidator<ReplaceStackResourceBindings>
    {
        public Validator()
        {
            RuleFor(x => x.Entries).NotNull();
            RuleForEach(x => x.Entries).SetValidator(new ResourceBindingInputValidator(allowMountedFile: true));
        }
    }
}

[RequirePermission(ResourceType.Deployment, PermissionLevel.Write, SpecificPermission.ResourceBindings)]
public sealed record ReplaceDeploymentResourceBindings(Guid Id, IReadOnlyList<ResourceBindingInput> Entries) : ICommand<Result<ResourceBindingsResult>>
{
    internal sealed class Validator : AbstractValidator<ReplaceDeploymentResourceBindings>
    {
        public Validator()
        {
            RuleFor(x => x.Entries).NotNull();
            RuleForEach(x => x.Entries).SetValidator(new ResourceBindingInputValidator(allowMountedFile: false));
        }
    }
}

internal sealed class ReplaceGlobalResourceBindingsHandler(IUnitOfWork unitOfWork)
    : ICommandHandler<ReplaceGlobalResourceBindings, Result<ResourceBindingsResult>>
{
    public async ValueTask<Result<ResourceBindingsResult>> Handle(ReplaceGlobalResourceBindings command, CancellationToken cancellationToken)
        => await ResourceBindingsFeatureHelpers.ReplaceEntriesAsync(
            unitOfWork,
            ResourceBindingScope.Global,
            null,
            command.Entries,
            cancellationToken);
}

internal sealed class ReplaceStackResourceBindingsHandler(IUnitOfWork unitOfWork)
    : ICommandHandler<ReplaceStackResourceBindings, Result<ResourceBindingsResult>>
{
    public async ValueTask<Result<ResourceBindingsResult>> Handle(ReplaceStackResourceBindings command, CancellationToken cancellationToken)
    {
        var stack = await unitOfWork.Stacks.GetAsync(command.Id, cancellationToken);
        if (stack is null)
            return Result.Failure<ResourceBindingsResult>(new NotFoundError($"Stack with ID {command.Id} does not exist."));

        return await ResourceBindingsFeatureHelpers.ReplaceEntriesAsync(
            unitOfWork,
            ResourceBindingScope.Stack,
            command.Id,
            command.Entries,
            cancellationToken);
    }
}

internal sealed class ReplaceDeploymentResourceBindingsHandler(IUnitOfWork unitOfWork)
    : ICommandHandler<ReplaceDeploymentResourceBindings, Result<ResourceBindingsResult>>
{
    public async ValueTask<Result<ResourceBindingsResult>> Handle(ReplaceDeploymentResourceBindings command, CancellationToken cancellationToken)
    {
        var deployment = await unitOfWork.Deployments.GetAsync(command.Id, cancellationToken);
        if (deployment is null)
            return Result.Failure<ResourceBindingsResult>(new NotFoundError($"Deployment with ID {command.Id} does not exist."));

        return await ResourceBindingsFeatureHelpers.ReplaceEntriesAsync(
            unitOfWork,
            ResourceBindingScope.Deployment,
            command.Id,
            command.Entries,
            cancellationToken);
    }
}
