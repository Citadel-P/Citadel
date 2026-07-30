using Application.Services;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.ResourceBindings;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.ResourceBindings.Commands;

[RequirePermission(ResourceType.Binding, PermissionLevel.Write)]
public sealed record CreateInternalSecret(string Name, string Value) : ICommand<Result<SecretDefinition>>
{
    internal sealed class Validator : AbstractValidator<CreateInternalSecret>
    {
        public Validator()
        {
            RuleFor(x => x.Name)
                .NotEmpty()
                .MaximumLength(128)
                .Matches("^[A-Za-z_][A-Za-z0-9_]*$")
                .WithMessage("Secret name must be a valid environment variable name.");
            RuleFor(x => x.Value).NotNull();
        }
    }
}

[RequirePermission(
    ResourceType.Stack,
    PermissionLevel.Write,
    SpecificPermission.ResourceBindings,
    ResourceIdProperty = nameof(CreateStackInternalSecret.Id))]
public sealed record CreateStackInternalSecret(Guid Id, string Name, string Value) : ICommand<Result<SecretDefinition>>
{
    internal sealed class Validator : AbstractValidator<CreateStackInternalSecret>
    {
        public Validator()
        {
            RuleFor(x => x.Name)
                .NotEmpty()
                .MaximumLength(128)
                .Matches("^[A-Za-z_][A-Za-z0-9_]*$")
                .WithMessage("Secret name must be a valid environment variable name.");
            RuleFor(x => x.Value).NotNull();
        }
    }
}

[RequirePermission(
    ResourceType.Deployment,
    PermissionLevel.Write,
    SpecificPermission.ResourceBindings,
    ResourceIdProperty = nameof(CreateDeploymentInternalSecret.Id))]
public sealed record CreateDeploymentInternalSecret(Guid Id, string Name, string Value) : ICommand<Result<SecretDefinition>>
{
    internal sealed class Validator : AbstractValidator<CreateDeploymentInternalSecret>
    {
        public Validator()
        {
            RuleFor(x => x.Name)
                .NotEmpty()
                .MaximumLength(128)
                .Matches("^[A-Za-z_][A-Za-z0-9_]*$")
                .WithMessage("Secret name must be a valid environment variable name.");
            RuleFor(x => x.Value).NotNull();
        }
    }
}

internal sealed class CreateInternalSecretHandler(IUnitOfWork unitOfWork, ISecretValueProtector secretValueProtector)
    : ICommandHandler<CreateInternalSecret, Result<SecretDefinition>>,
      ICommandHandler<CreateStackInternalSecret, Result<SecretDefinition>>,
      ICommandHandler<CreateDeploymentInternalSecret, Result<SecretDefinition>>
{
    public async ValueTask<Result<SecretDefinition>> Handle(CreateInternalSecret command, CancellationToken cancellationToken)
        => await CreateAsync(command.Name, command.Value, cancellationToken);

    public async ValueTask<Result<SecretDefinition>> Handle(
        CreateStackInternalSecret command,
        CancellationToken cancellationToken)
    {
        if (await unitOfWork.Stacks.GetAsync(command.Id, cancellationToken) is null)
            return Result.Failure<SecretDefinition>(new NotFoundError($"Stack with ID {command.Id} does not exist."));

        return await CreateAsync(command.Name, command.Value, cancellationToken);
    }

    public async ValueTask<Result<SecretDefinition>> Handle(
        CreateDeploymentInternalSecret command,
        CancellationToken cancellationToken)
    {
        if (await unitOfWork.Deployments.GetAsync(command.Id, cancellationToken) is null)
            return Result.Failure<SecretDefinition>(new NotFoundError($"Deployment with ID {command.Id} does not exist."));

        return await CreateAsync(command.Name, command.Value, cancellationToken);
    }

    private async ValueTask<Result<SecretDefinition>> CreateAsync(
        string name,
        string value,
        CancellationToken cancellationToken)
    {
        if (await unitOfWork.SecretDefinitions.ExistsByNameAsync(name, cancellationToken))
            return Result.Failure<SecretDefinition>(new ConflictError("Name already exists"));

        var secret = new SecretDefinition(name, SecretProviderType.InternalEncrypted);
        try
        {
            secret.Validate();
        }
        catch (ArgumentException ex)
        {
            return Result.Failure<SecretDefinition>(new BadRequestError(ex.Message));
        }

        var secretValue = new InternalSecretValue(secret.Id, secretValueProtector.Protect(value));
        await unitOfWork.SecretDefinitions.AddAsync(secret, secretValue, cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);

        return secret;
    }
}
