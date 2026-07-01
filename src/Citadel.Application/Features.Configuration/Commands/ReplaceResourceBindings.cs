using Application.Features.Configuration.Models;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.ResourceBindings;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.Configuration.Commands;

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

    internal sealed class ResourceBindingInputValidator : AbstractValidator<ResourceBindingInput>
    {
        public ResourceBindingInputValidator(bool allowMountedFile)
        {
            RuleFor(x => x.Name)
                .NotEmpty()
                .MaximumLength(128)
                .Matches("^[A-Za-z_][A-Za-z0-9_]*$")
                .WithMessage("Resource binding name must be a valid environment variable name.");

            When(x => x.Kind == ResourceBindingKind.Variable, () =>
            {
                RuleFor(x => x.Value).NotNull();
                RuleFor(x => x.SecretId).Null();
                RuleFor(x => x.SecretDeliveryMode).Null();
                RuleFor(x => x.TargetPath).Null();
            });

            When(x => x.Kind == ResourceBindingKind.Secret, () =>
            {
                RuleFor(x => x.Value).Null();
                RuleFor(x => x.SecretId).NotNull();
                RuleFor(x => x.SecretDeliveryMode).NotNull();
                When(x => x.SecretDeliveryMode == SecretDeliveryMode.EnvironmentVariable, () =>
                {
                    RuleFor(x => x.TargetPath).Null();
                });
                When(x => x.SecretDeliveryMode == SecretDeliveryMode.MountedFile, () =>
                {
                    if (allowMountedFile)
                    {
                        RuleFor(x => x.TargetPath)
                            .NotEmpty()
                            .Must(BeValidMountedFileTargetPath)
                            .WithMessage("Mounted file target path must be an absolute Linux file path outside protected system paths.");
                    }
                    else
                    {
                        RuleFor(x => x.SecretDeliveryMode)
                            .Equal(SecretDeliveryMode.EnvironmentVariable)
                            .WithMessage("Only environment variable secret delivery is supported.");
                    }
                });
                When(x => x.SecretDeliveryMode == SecretDeliveryMode.NativePlatformSecret, () =>
                {
                    RuleFor(x => x.SecretDeliveryMode)
                        .Equal(SecretDeliveryMode.EnvironmentVariable)
                        .WithMessage("Native platform secret delivery is not supported.");
                });
            });
        }

        private static bool BeValidMountedFileTargetPath(string? targetPath)
        {
            try
            {
                new ResourceBinding(
                    Name: "SECRET",
                    Kind: ResourceBindingKind.Secret,
                    Scope: ResourceBindingScope.Stack,
                    ResourceId: Guid.CreateVersion7(),
                    Value: null,
                    SecretId: Guid.CreateVersion7(),
                    SecretDeliveryMode: SecretDeliveryMode.MountedFile,
                    TargetPath: targetPath).Validate();
                return true;
            }
            catch (ArgumentException)
            {
                return false;
            }
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
            RuleForEach(x => x.Entries).SetValidator(new ReplaceGlobalResourceBindings.ResourceBindingInputValidator(allowMountedFile: true));
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
            RuleForEach(x => x.Entries).SetValidator(new ReplaceGlobalResourceBindings.ResourceBindingInputValidator(allowMountedFile: false));
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
