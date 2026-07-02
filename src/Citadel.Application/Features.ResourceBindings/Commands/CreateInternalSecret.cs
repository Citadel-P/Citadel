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

internal sealed class CreateInternalSecretHandler(IUnitOfWork unitOfWork, ISecretValueProtector secretValueProtector)
    : ICommandHandler<CreateInternalSecret, Result<SecretDefinition>>
{
    public async ValueTask<Result<SecretDefinition>> Handle(CreateInternalSecret command, CancellationToken cancellationToken)
    {
        if (await unitOfWork.SecretDefinitions.ExistsByNameAsync(command.Name, cancellationToken))
            return Result.Failure<SecretDefinition>(new ConflictError("Name already exists"));

        var secret = new SecretDefinition(command.Name, SecretProviderType.InternalEncrypted);
        try
        {
            secret.Validate();
        }
        catch (ArgumentException ex)
        {
            return Result.Failure<SecretDefinition>(new BadRequestError(ex.Message));
        }

        var value = new InternalSecretValue(secret.Id, secretValueProtector.Protect(command.Value));
        await unitOfWork.SecretDefinitions.AddAsync(secret, value, cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);

        return secret;
    }
}
