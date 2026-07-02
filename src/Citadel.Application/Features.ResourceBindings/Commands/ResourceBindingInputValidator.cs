using Application.Features.ResourceBindings.Models;
using Domain;
using Domain.Entities.ResourceBindings;
using FluentValidation;

namespace Application.Features.ResourceBindings.Commands;

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
