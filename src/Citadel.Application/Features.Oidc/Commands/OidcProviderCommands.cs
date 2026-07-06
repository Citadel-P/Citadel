using Application.Features.Oidc.Models;
using Application.Services;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Activities;
using Domain.Entities.Oidc;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.Abstraction;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.Oidc.Commands;

[RequirePermission(ResourceType.Binding, PermissionLevel.Write)]
public sealed record CreateOidcProvider(OidcProviderInputModel Provider) : ICommand<Result<OidcProvider>>
{
    internal sealed class Validator : AbstractValidator<CreateOidcProvider>
    {
        public Validator()
        {
            RuleFor(x => x.Provider).SetValidator(new OidcProviderInputModelValidator());
        }
    }
}

[RequirePermission(ResourceType.Binding, PermissionLevel.Write)]
public sealed record UpdateOidcProvider(
    Guid Id,
    UpdateOidcProviderInputModel Provider,
    bool UpdateDescription,
    bool UpdateAllowedEmailDomains,
    bool UpdateRequiredClaimName,
    bool UpdateRequiredClaimValues,
    bool UpdateDefaultRoleId) : ICommand<Result<OidcProvider>>
{
    internal sealed class Validator : AbstractValidator<UpdateOidcProvider>
    {
        public Validator()
        {
            RuleFor(x => x.Id).NotEmpty();
            RuleFor(x => x.Provider).SetValidator(new UpdateOidcProviderInputModelValidator());
        }
    }
}

[RequirePermission(ResourceType.Binding, PermissionLevel.Write)]
public sealed record DeleteOidcProvider(Guid Id) : ICommand<Result>;

[RequirePermission(ResourceType.Binding, PermissionLevel.Read)]
public sealed record TestOidcProviderDiscovery(Guid? ProviderId, string? Issuer) : ICommand<Result<OidcDiscoveryResult>>;

internal sealed class CreateOidcProviderHandler(
    IUnitOfWork unitOfWork,
    ISecretValueProtector secretValueProtector,
    IUserContextAccessor userContextAccessor)
    : ICommandHandler<CreateOidcProvider, Result<OidcProvider>>
{
    public async ValueTask<Result<OidcProvider>> Handle(CreateOidcProvider command, CancellationToken cancellationToken)
    {
        var input = command.Provider;
        if (await unitOfWork.OidcProviders.ExistsByNameAsync(input.Name, cancellationToken))
            return Result.Failure<OidcProvider>(new ConflictError("OIDC provider name already exists."));

        if (input.DefaultRoleId.HasValue && await unitOfWork.Roles.GetAsync(input.DefaultRoleId.Value, cancellationToken) is null)
            return Result.Failure<OidcProvider>(new BadRequestError("Default role does not exist."));

        var provider = new OidcProvider(
            input.Name,
            input.Description,
            input.DisplayName,
            input.Issuer,
            input.ClientId,
            ProtectSecret(secretValueProtector, input.ClientSecret),
            input.Scopes ?? "openid profile email",
            input.Enabled,
            input.AutoProvisionUsers,
            input.AllowEmailAutoLink,
            input.RequireEmailVerified,
            input.AllowedEmailDomains,
            input.RequiredClaimName,
            input.RequiredClaimValues,
            input.DefaultRoleId,
            userContextAccessor.Current.ActorId);

        try
        {
            provider.Validate();
        }
        catch (ArgumentException ex)
        {
            return Result.Failure<OidcProvider>(new BadRequestError(ex.Message));
        }

        await unitOfWork.OidcProviders.AddAsync(provider, cancellationToken);
        await unitOfWork.ActivityEventRepository.AddAsync(
            new ActivityEvent(
                platformId: null,
                resourceId: provider.Id,
                actorId: userContextAccessor.Current.ActorId,
                resourceName: provider.Name,
                eventType: ActivityEventType.OidcProviderCreated,
                status: ActivityStatus.Success,
                info: new OidcProviderCreated(OidcProviderActivity.ToSnapshot(provider))),
            cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);

        return Result.Success(provider);
    }

    private static string? ProtectSecret(ISecretValueProtector protector, string? secret)
        => string.IsNullOrWhiteSpace(secret) ? null : protector.Protect(secret);
}

internal sealed class UpdateOidcProviderHandler(
    IUnitOfWork unitOfWork,
    ISecretValueProtector secretValueProtector,
    IUserContextAccessor userContextAccessor)
    : ICommandHandler<UpdateOidcProvider, Result<OidcProvider>>
{
    public async ValueTask<Result<OidcProvider>> Handle(UpdateOidcProvider command, CancellationToken cancellationToken)
    {
        var existing = await unitOfWork.OidcProviders.GetAsync(command.Id, cancellationToken);
        if (existing is null)
            return Result.Failure<OidcProvider>(new NotFoundError("OIDC provider not found."));

        var input = command.Provider;
        var oldProvider = OidcProviderActivity.ToSnapshot(existing);
        var name = input.Name ?? existing.Name;
        if (await unitOfWork.OidcProviders.ExistsByNameExceptAsync(name, command.Id, cancellationToken))
            return Result.Failure<OidcProvider>(new ConflictError("OIDC provider name already exists."));

        if (command.UpdateDefaultRoleId
            && input.DefaultRoleId.HasValue
            && input.DefaultRoleId.Value != Guid.Empty
            && await unitOfWork.Roles.GetAsync(input.DefaultRoleId.Value, cancellationToken) is null)
        {
            return Result.Failure<OidcProvider>(new BadRequestError("Default role does not exist."));
        }

        var protectedSecret = string.IsNullOrWhiteSpace(input.ClientSecret)
            ? null
            : secretValueProtector.Protect(input.ClientSecret);

        existing.Update(
            input.Name,
            input.Description,
            command.UpdateDescription,
            input.DisplayName,
            input.Issuer,
            input.ClientId,
            protectedSecret,
            input.Scopes,
            input.Enabled,
            input.AutoProvisionUsers,
            input.AllowEmailAutoLink,
            input.RequireEmailVerified,
            input.AllowedEmailDomains,
            command.UpdateAllowedEmailDomains,
            input.RequiredClaimName,
            command.UpdateRequiredClaimName,
            input.RequiredClaimValues,
            command.UpdateRequiredClaimValues,
            input.DefaultRoleId,
            command.UpdateDefaultRoleId);

        try
        {
            existing.Validate();
        }
        catch (ArgumentException ex)
        {
            return Result.Failure<OidcProvider>(new BadRequestError(ex.Message));
        }

        await unitOfWork.OidcProviders.UpdateAsync(existing, cancellationToken);
        await unitOfWork.ActivityEventRepository.AddAsync(
            new ActivityEvent(
                platformId: null,
                resourceId: existing.Id,
                actorId: userContextAccessor.Current.ActorId,
                resourceName: existing.Name,
                eventType: ActivityEventType.OidcProviderUpdated,
                status: ActivityStatus.Success,
                info: new OidcProviderUpdated(oldProvider, OidcProviderActivity.ToSnapshot(existing))),
            cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);

        return Result.Success(existing);
    }
}

internal sealed class DeleteOidcProviderHandler(IUnitOfWork unitOfWork, IUserContextAccessor userContextAccessor)
    : ICommandHandler<DeleteOidcProvider, Result>
{
    public async ValueTask<Result> Handle(DeleteOidcProvider command, CancellationToken cancellationToken)
    {
        var existing = await unitOfWork.OidcProviders.GetAsync(command.Id, cancellationToken);
        if (existing is null)
            return Result.Failure(new NotFoundError("OIDC provider not found."));

        await unitOfWork.OidcProviders.DeleteAsync(command.Id, cancellationToken);
        await unitOfWork.ActivityEventRepository.AddAsync(
            new ActivityEvent(
                platformId: null,
                resourceId: existing.Id,
                actorId: userContextAccessor.Current.ActorId,
                resourceName: existing.Name,
                eventType: ActivityEventType.OidcProviderDeleted,
                status: ActivityStatus.Success,
                info: new OidcProviderDeleted(OidcProviderActivity.ToSnapshot(existing))),
            cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);

        return Result.Success();
    }
}

internal static class OidcProviderActivity
{
    internal static OidcProviderActivitySnapshot ToSnapshot(OidcProvider provider)
        => new(
            provider.Id,
            provider.Name,
            provider.Description,
            provider.DisplayName,
            provider.Issuer,
            provider.ClientId,
            provider.Scopes,
            provider.Enabled,
            provider.AutoProvisionUsers,
            provider.AllowEmailAutoLink,
            provider.RequireEmailVerified,
            provider.AllowedEmailDomains,
            provider.RequiredClaimName,
            provider.RequiredClaimValues,
            provider.DefaultRoleId);
}

internal sealed class TestOidcProviderDiscoveryHandler(
    IUnitOfWork unitOfWork,
    IOidcDiscoveryService discoveryService)
    : ICommandHandler<TestOidcProviderDiscovery, Result<OidcDiscoveryResult>>
{
    public async ValueTask<Result<OidcDiscoveryResult>> Handle(TestOidcProviderDiscovery command, CancellationToken cancellationToken)
    {
        var issuer = command.Issuer;
        if (string.IsNullOrWhiteSpace(issuer) && command.ProviderId.HasValue)
        {
            var existing = await unitOfWork.OidcProviders.GetAsync(command.ProviderId.Value, cancellationToken);
            if (existing is null)
                return Result.Failure<OidcDiscoveryResult>(new NotFoundError("OIDC provider not found."));

            issuer = existing.Issuer;
        }

        if (string.IsNullOrWhiteSpace(issuer))
            return Result.Failure<OidcDiscoveryResult>(new BadRequestError("Issuer is required."));

        return await discoveryService.TestDiscoveryAsync(issuer, cancellationToken);
    }
}

internal sealed class OidcProviderInputModelValidator : AbstractValidator<OidcProviderInputModel>
{
    public OidcProviderInputModelValidator()
    {
        RuleFor(x => x.Name).NotEmpty().MaximumLength(128);
        RuleFor(x => x.Description).MaximumLength(600).When(x => x.Description is not null);
        RuleFor(x => x.DisplayName).NotEmpty().MaximumLength(128);
        RuleFor(x => x.Issuer).NotEmpty().MaximumLength(512);
        RuleFor(x => x.ClientId).NotEmpty().MaximumLength(256);
        RuleFor(x => x.ClientSecret).MaximumLength(4096).When(x => x.ClientSecret is not null);
        RuleFor(x => x.Scopes).MaximumLength(512).When(x => x.Scopes is not null);
        RuleFor(x => x.AllowedEmailDomains).MaximumLength(1024).When(x => x.AllowedEmailDomains is not null);
        RuleFor(x => x.RequiredClaimName).MaximumLength(256).When(x => x.RequiredClaimName is not null);
        RuleFor(x => x.RequiredClaimValues).MaximumLength(1024).When(x => x.RequiredClaimValues is not null);
    }
}

internal sealed class UpdateOidcProviderInputModelValidator : AbstractValidator<UpdateOidcProviderInputModel>
{
    public UpdateOidcProviderInputModelValidator()
    {
        RuleFor(x => x.Name).NotEmpty().MaximumLength(128).When(x => x.Name is not null);
        RuleFor(x => x.Description).MaximumLength(600).When(x => x.Description is not null);
        RuleFor(x => x.DisplayName).NotEmpty().MaximumLength(128).When(x => x.DisplayName is not null);
        RuleFor(x => x.Issuer).NotEmpty().MaximumLength(512).When(x => x.Issuer is not null);
        RuleFor(x => x.ClientId).NotEmpty().MaximumLength(256).When(x => x.ClientId is not null);
        RuleFor(x => x.ClientSecret).MaximumLength(4096).When(x => x.ClientSecret is not null);
        RuleFor(x => x.Scopes).MaximumLength(512).When(x => x.Scopes is not null);
        RuleFor(x => x.AllowedEmailDomains).MaximumLength(1024).When(x => x.AllowedEmailDomains is not null);
        RuleFor(x => x.RequiredClaimName).MaximumLength(256).When(x => x.RequiredClaimName is not null);
        RuleFor(x => x.RequiredClaimValues).MaximumLength(1024).When(x => x.RequiredClaimValues is not null);
    }
}
