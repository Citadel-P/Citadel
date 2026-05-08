using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Registries;
using FluentValidation;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using Hosting.Common.MergePatch;
using LightResults;
using Mediator;
using Hosting.Common;

namespace Application.Features.Registries.Commands;

[RequirePermission(ResourceType.Registry, PermissionLevel.Write)]
public sealed record PatchRegistryMetadata(Guid Id, JsonMergePatchDocument<Registry> Patch) : ICommand<Result<Registry>>
{
    internal sealed class Validator : PatchCommandValidator<PatchRegistryMetadata, Registry>
    {
        public Validator()
            : base(
                  patchSelector: x => x.Patch,
                  jsonTypeInfo: RegistryJsonContext.Default.Registry,
                  modelValidator: new RegistryValidator())
        { }
    }

    internal sealed class RegistryValidator : AbstractValidator<Registry>
    {
        public RegistryValidator()
        {
            RuleFor(x => x.Id).NotEmpty().NotNull();
            When(s => s.Description != null, () => RuleFor(x => x.Description).MaximumLength(600));
        }
    }
}

internal sealed class PatchRegistryMetadataHandler(IUnitOfWork unitOfWork) : ICommandHandler<PatchRegistryMetadata, Result<Registry>>
{
    public async ValueTask<Result<Registry>> Handle(PatchRegistryMetadata command, CancellationToken cancellationToken)
    {
        var registry = await unitOfWork.Registries.GetAsync(command.Id, cancellationToken);
        if (registry == null)
        {
            return Result.Failure<Registry>(new NotFoundError("The provided registry does not exist"));
        }

        var patchedRegistry = command.Patch.ApplyTo(registry, RegistryJsonContext.Default.Registry);

        registry.PartialUpdate(description: patchedRegistry.Description);

        await unitOfWork.Registries.UpdateAsync(registry, cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);

        return registry;
    }
}
