using Application.Features.Configuration.Models;
using Application.Features.Configuration;
using Domain.Contracts.Interfaces;
using Domain.Entities.Configuration;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;
using Domain;

namespace Application.Features.Configuration.Commands;

[RequirePermission(ResourceType.Configuration, PermissionLevel.Write)]
public sealed record ReplaceGlobalConfigurationEntries(IReadOnlyList<ConfigurationEntryInput> Entries) : ICommand<Result<ConfigurationEntriesResult>>
{
    internal sealed class Validator : AbstractValidator<ReplaceGlobalConfigurationEntries>
    {
        public Validator()
        {
            RuleFor(x => x.Entries).NotNull();
            RuleForEach(x => x.Entries).SetValidator(new ConfigurationEntryInputValidator(allowMountedFile: false));
        }
    }

    internal sealed class ConfigurationEntryInputValidator : AbstractValidator<ConfigurationEntryInput>
    {
        public ConfigurationEntryInputValidator(bool allowMountedFile)
        {
            RuleFor(x => x.Name)
                .NotEmpty()
                .MaximumLength(128)
                .Matches("^[A-Za-z_][A-Za-z0-9_]*$")
                .WithMessage("Configuration entry name must be a valid environment variable name.");

            When(x => x.Kind == ConfigurationEntryKind.Variable, () =>
            {
                RuleFor(x => x.Value).NotNull();
                RuleFor(x => x.SecretId).Null();
                RuleFor(x => x.SecretDeliveryMode).Null();
                RuleFor(x => x.TargetPath).Null();
            });

            When(x => x.Kind == ConfigurationEntryKind.Secret, () =>
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
                new ConfigurationEntry(
                    Name: "SECRET",
                    Kind: ConfigurationEntryKind.Secret,
                    Scope: ConfigurationScope.Stack,
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

[RequirePermission(ResourceType.Stack, PermissionLevel.Write, SpecificPermission.Configuration)]
public sealed record ReplaceStackConfigurationEntries(Guid Id, IReadOnlyList<ConfigurationEntryInput> Entries) : ICommand<Result<ConfigurationEntriesResult>>
{
    internal sealed class Validator : AbstractValidator<ReplaceStackConfigurationEntries>
    {
        public Validator()
        {
            RuleFor(x => x.Entries).NotNull();
            RuleForEach(x => x.Entries).SetValidator(new ReplaceGlobalConfigurationEntries.ConfigurationEntryInputValidator(allowMountedFile: true));
        }
    }
}

[RequirePermission(ResourceType.Deployment, PermissionLevel.Write, SpecificPermission.Configuration)]
public sealed record ReplaceDeploymentConfigurationEntries(Guid Id, IReadOnlyList<ConfigurationEntryInput> Entries) : ICommand<Result<ConfigurationEntriesResult>>
{
    internal sealed class Validator : AbstractValidator<ReplaceDeploymentConfigurationEntries>
    {
        public Validator()
        {
            RuleFor(x => x.Entries).NotNull();
            RuleForEach(x => x.Entries).SetValidator(new ReplaceGlobalConfigurationEntries.ConfigurationEntryInputValidator(allowMountedFile: false));
        }
    }
}

internal sealed class ReplaceGlobalConfigurationEntriesHandler(IUnitOfWork unitOfWork)
    : ICommandHandler<ReplaceGlobalConfigurationEntries, Result<ConfigurationEntriesResult>>
{
    public async ValueTask<Result<ConfigurationEntriesResult>> Handle(ReplaceGlobalConfigurationEntries command, CancellationToken cancellationToken)
        => await ConfigurationEntriesFeatureHelpers.ReplaceEntriesAsync(
            unitOfWork,
            ConfigurationScope.Global,
            null,
            command.Entries,
            cancellationToken);
}

internal sealed class ReplaceStackConfigurationEntriesHandler(IUnitOfWork unitOfWork)
    : ICommandHandler<ReplaceStackConfigurationEntries, Result<ConfigurationEntriesResult>>
{
    public async ValueTask<Result<ConfigurationEntriesResult>> Handle(ReplaceStackConfigurationEntries command, CancellationToken cancellationToken)
    {
        var stack = await unitOfWork.Stacks.GetAsync(command.Id, cancellationToken);
        if (stack is null)
            return Result.Failure<ConfigurationEntriesResult>(new NotFoundError($"Stack with ID {command.Id} does not exist."));

        return await ConfigurationEntriesFeatureHelpers.ReplaceEntriesAsync(
            unitOfWork,
            ConfigurationScope.Stack,
            command.Id,
            command.Entries,
            cancellationToken);
    }
}

internal sealed class ReplaceDeploymentConfigurationEntriesHandler(IUnitOfWork unitOfWork)
    : ICommandHandler<ReplaceDeploymentConfigurationEntries, Result<ConfigurationEntriesResult>>
{
    public async ValueTask<Result<ConfigurationEntriesResult>> Handle(ReplaceDeploymentConfigurationEntries command, CancellationToken cancellationToken)
    {
        var deployment = await unitOfWork.Deployments.GetAsync(command.Id, cancellationToken);
        if (deployment is null)
            return Result.Failure<ConfigurationEntriesResult>(new NotFoundError($"Deployment with ID {command.Id} does not exist."));

        return await ConfigurationEntriesFeatureHelpers.ReplaceEntriesAsync(
            unitOfWork,
            ConfigurationScope.Deployment,
            command.Id,
            command.Entries,
            cancellationToken);
    }
}
