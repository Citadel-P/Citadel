using System.Security.Cryptography;
using System.Text.RegularExpressions;
using Application.Features.Identity.Auth.Models;
using Application.Services;
using Application.Services.Identity;
using Application.Services.Licensing;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Identity;
using Domain.Contracts.Resources.Oidc;
using Domain.Entities.Identity;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;
using static Application.Features.Oidc.Commands.OidcOpaqueValue;

namespace Application.Features.Oidc.Commands;

public sealed record CompleteOidcLogin(Guid ProviderId, string Code, string State, string RedirectUri) : ICommand<Result<OidcLoginCompleteResult>>
{
    internal sealed class Validator : AbstractValidator<CompleteOidcLogin>
    {
        public Validator()
        {
            RuleFor(x => x.ProviderId).NotEmpty();
            RuleFor(x => x.Code).NotEmpty();
            RuleFor(x => x.State).NotEmpty();
            RuleFor(x => x.RedirectUri).NotEmpty();
        }
    }
}

internal sealed class CompleteOidcLoginHandler(
    IUnitOfWork unitOfWork,
    ISecretValueProtector secretValueProtector,
    IOidcDiscoveryService discoveryService,
    IOidcAuthenticationService authenticationService,
    IJwtService jwtService,
    IRoleCache roleCache,
    IActorScopeEvictor actorScopeEvictor,
    ILicenseQuotaService licenseQuotaService,
    IRequestSessionMetadataAccessor requestSessionMetadataAccessor,
    IRefreshTokenCookieService refreshTokenCookieService)
    : ICommandHandler<CompleteOidcLogin, Result<OidcLoginCompleteResult>>
{
    public async ValueTask<Result<OidcLoginCompleteResult>> Handle(CompleteOidcLogin command, CancellationToken cancellationToken)
    {
        var provider = await unitOfWork.OidcProviders.GetAsync(command.ProviderId, cancellationToken);
        if (provider is null || !provider.Enabled)
            return Result.Failure<OidcLoginCompleteResult>(new NotFoundError("OIDC provider not found."));

        var state = await unitOfWork.OidcLoginStates.GetByStateHashAsync(HashOpaqueValue(command.State), cancellationToken);
        if (state is null || state.ProviderId != provider.Id || state.ExpiresAt <= DateTime.UtcNow)
            return Result.Failure<OidcLoginCompleteResult>(new BadRequestError("OIDC login state is invalid or expired."));

        await unitOfWork.OidcLoginStates.DeleteAsync(state.Id, cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);

        var discovery = await discoveryService.GetDiscoveryAsync(provider.Issuer, cancellationToken);
        if (!discovery.IsSuccess(out var discoveryResult))
            return Result.Failure<OidcLoginCompleteResult>(discovery.Errors);

        var clientSecret = UnprotectSecret(provider.ClientSecretCiphertext);
        if (clientSecret is null && !string.IsNullOrWhiteSpace(provider.ClientSecretCiphertext))
            return Result.Failure<OidcLoginCompleteResult>(new BadRequestError("OIDC provider client secret could not be decrypted."));

        var identity = await authenticationService.ExchangeAndValidateAsync(
            provider,
            discoveryResult,
            clientSecret ?? string.Empty,
            command.Code,
            command.RedirectUri,
            state.CodeVerifier,
            state.Nonce,
            cancellationToken);
        if (!identity.IsSuccess(out var oidcIdentity))
            return Result.Failure<OidcLoginCompleteResult>(identity.Errors);

        var policyResult = ValidateProviderPolicy(provider, oidcIdentity);
        if (!policyResult.IsSuccess())
            return Result.Failure<OidcLoginCompleteResult>(policyResult.Errors);

        var userAuthInfo = await ResolveUserAsync(provider, oidcIdentity, cancellationToken);
        if (!userAuthInfo.IsSuccess(out var authInfo))
            return Result.Failure<OidcLoginCompleteResult>(userAuthInfo.Errors);

        var accessToken = jwtService.CreateAccessToken(User.GetJwtClaims(authInfo));
        var (refreshTokenId, refreshToken, refreshTokenExpiresAt) = jwtService.CreateRefreshToken();
        var metadata = requestSessionMetadataAccessor.GetCurrent();

        await unitOfWork.RefreshTokens.AddAsync(
            RefreshToken.Create(
                refreshTokenId,
                authInfo.Id,
                refreshTokenExpiresAt,
                metadata.UserAgent,
                metadata.IpAddress),
            cancellationToken);
        var tokensCount = await unitOfWork.RefreshTokens.CountAsync(authInfo.Id, cancellationToken);
        const int maxTokensPerUser = 10;
        if (tokensCount > maxTokensPerUser)
            await unitOfWork.RefreshTokens.DeleteOldestTokensAsync(authInfo.Id, tokensCount - maxTokensPerUser, cancellationToken);

        await unitOfWork.CommitAsync(cancellationToken);
        refreshTokenCookieService.Set(refreshToken, refreshTokenExpiresAt);
        roleCache.SetRoles(authInfo.Id, authInfo.Roles);

        return Result.Success(new OidcLoginCompleteResult(new LoginResponse(accessToken, LoginNextStep.Completed), state.ReturnUrl));
    }

    private async Task<Result<UserAuthInfo>> ResolveUserAsync(
        Domain.Entities.Oidc.OidcProvider provider,
        OidcTokenIdentity identity,
        CancellationToken cancellationToken)
    {
        var now = DateTime.UtcNow;
        var externalLogin = await unitOfWork.OidcExternalLogins.GetAsync(provider.Id, identity.Subject, cancellationToken);
        if (externalLogin is not null)
        {
            await unitOfWork.OidcExternalLogins.UpdateSeenAsync(externalLogin.Id, identity.Email, now, cancellationToken);
            var linkedUser = await unitOfWork.Users.GetUserAuthInfoByIdAsync(externalLogin.UserId, cancellationToken);
            return linkedUser is null
                ? Result.Failure<UserAuthInfo>(new NotFoundError("Linked OIDC user no longer exists or is disabled."))
                : Result.Success(linkedUser);
        }

        UserAuthInfo? userAuthInfo = null;
        if (provider.AllowEmailAutoLink && !string.IsNullOrWhiteSpace(identity.Email))
            userAuthInfo = await unitOfWork.Users.GetUserAuthInfoByEmailAsync(identity.Email, cancellationToken);

        if (userAuthInfo is null)
        {
            if (!provider.AutoProvisionUsers)
                return Result.Failure<UserAuthInfo>(new NotFoundError("No Citadel user is linked to this OIDC account."));

            if (string.IsNullOrWhiteSpace(identity.Email))
                return Result.Failure<UserAuthInfo>(new BadRequestError("OIDC user provisioning requires an email claim."));

            var provisionResult = await ProvisionUserAsync(provider, identity, cancellationToken);
            if (!provisionResult.IsSuccess(out userAuthInfo))
                return Result.Failure<UserAuthInfo>(provisionResult.Errors);
        }

        await unitOfWork.OidcExternalLogins.AddAsync(
            new OidcExternalLogin(
                Guid.CreateVersion7(),
                provider.Id,
                identity.Subject,
                userAuthInfo.Id,
                identity.Email,
                now,
                now),
            cancellationToken);

        return Result.Success(userAuthInfo);
    }

    private async Task<Result<UserAuthInfo>> ProvisionUserAsync(
        Domain.Entities.Oidc.OidcProvider provider,
        OidcTokenIdentity identity,
        CancellationToken cancellationToken)
    {
        var quotaResult = await licenseQuotaService.EnsureCanIncreaseAsync(
            new Dictionary<LicenseLimit, int> { [LicenseLimit.ActiveUsers] = 1 },
            unitOfWork,
            cancellationToken);
        if (!quotaResult.IsSuccess())
            return Result.Failure<UserAuthInfo>(quotaResult.Errors);

        var name = await CreateUniqueUserNameAsync(identity, cancellationToken);
        var actor = Actor.Create(ActorType.User, new ActorMetadata(name));
        var password = Convert.ToBase64String(RandomNumberGenerator.GetBytes(32));
        var user = new User(name, identity.Email!, password, actor.Id, Constants.SystemId);

        await unitOfWork.Actors.AddAsync(actor, cancellationToken);
        await unitOfWork.Users.AddAsync(user, cancellationToken);

        if (provider.DefaultRoleId.HasValue)
        {
            await unitOfWork.Roles.AddActorRoleAsync(actor.Id, provider.DefaultRoleId.Value, cancellationToken);
            await actorScopeEvictor.EvictPermissionsForActorAsync(actor.Id, cancellationToken);
        }

        return Result.Success(
            await unitOfWork.Users.GetUserAuthInfoByIdAsync(user.Id, cancellationToken)
            ?? new UserAuthInfo(user.Id, user.ActorId, user.Name, user.Email, null, []));
    }

    private async Task<string> CreateUniqueUserNameAsync(OidcTokenIdentity identity, CancellationToken cancellationToken)
    {
        var source = identity.Name ?? identity.Email ?? identity.Subject;
        var baseName = Regex.Replace(source.Split('@')[0], "[^a-zA-Z0-9-_]", "-").Trim('-');
        if (baseName.Length < 3)
            baseName = $"oidc-{baseName}";

        baseName = baseName[..Math.Min(baseName.Length, 56)];
        var candidate = baseName;
        var suffix = 0;
        while (await unitOfWork.Users.ExistsByNameAsync(candidate, null, cancellationToken))
        {
            suffix++;
            candidate = $"{baseName}-{suffix}";
        }

        return candidate[..Math.Min(candidate.Length, 64)];
    }

    private string? UnprotectSecret(string? protectedSecret)
    {
        if (string.IsNullOrWhiteSpace(protectedSecret))
            return null;

        try
        {
            return secretValueProtector.Unprotect(protectedSecret);
        }
        catch (Exception)
        {
            return null;
        }
    }

    private static Result ValidateProviderPolicy(Domain.Entities.Oidc.OidcProvider provider, OidcTokenIdentity identity)
    {
        if (provider.RequireEmailVerified && !identity.EmailVerified)
            return Result.Failure(new BadRequestError("OIDC account email is not verified."));

        if (!string.IsNullOrWhiteSpace(provider.AllowedEmailDomains))
        {
            if (string.IsNullOrWhiteSpace(identity.Email) || !identity.Email.Contains('@', StringComparison.Ordinal))
                return Result.Failure(new BadRequestError("OIDC account email is required."));

            var domain = identity.Email.Split('@')[1];
            var allowed = provider.AllowedEmailDomains
                .Split(',', StringSplitOptions.RemoveEmptyEntries | StringSplitOptions.TrimEntries)
                .Any(value => string.Equals(value, domain, StringComparison.OrdinalIgnoreCase));
            if (!allowed)
                return Result.Failure(new BadRequestError("OIDC account email domain is not allowed."));
        }

        if (!string.IsNullOrWhiteSpace(provider.RequiredClaimName))
        {
            var allowedValues = (provider.RequiredClaimValues ?? string.Empty)
                .Split(',', StringSplitOptions.RemoveEmptyEntries | StringSplitOptions.TrimEntries)
                .ToHashSet(StringComparer.Ordinal);
            var hasRequiredClaim = identity.Claims
                .Where(claim => claim.Type == provider.RequiredClaimName)
                .Any(claim => allowedValues.Contains(claim.Value));
            if (!hasRequiredClaim)
                return Result.Failure(new BadRequestError("OIDC account is missing the required claim."));
        }

        return Result.Success();
    }
}
