using Application.Services;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Oidc;
using FluentValidation;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;
using static Application.Features.Oidc.Commands.OidcOpaqueValue;

namespace Application.Features.Oidc.Commands;

public sealed record BeginOidcLogin(
    Guid ProviderId,
    string RedirectUri,
    string ReturnUrl,
    IReadOnlyCollection<string> AllowedReturnOrigins)
    : ICommand<Result<OidcLoginStartResult>>
{
    internal sealed class Validator : AbstractValidator<BeginOidcLogin>
    {
        public Validator()
        {
            RuleFor(x => x.ProviderId).NotEmpty();
            RuleFor(x => x.RedirectUri).NotEmpty();
            RuleFor(x => x.ReturnUrl).NotEmpty();
        }
    }
}

internal sealed class BeginOidcLoginHandler(
    IUnitOfWork unitOfWork,
    IOidcDiscoveryService discoveryService,
    IOidcAuthenticationService authenticationService)
    : ICommandHandler<BeginOidcLogin, Result<OidcLoginStartResult>>
{
    public async ValueTask<Result<OidcLoginStartResult>> Handle(BeginOidcLogin command, CancellationToken cancellationToken)
    {
        var provider = await unitOfWork.OidcProviders.GetAsync(command.ProviderId, cancellationToken);
        if (provider is null || !provider.Enabled)
            return Result.Failure<OidcLoginStartResult>(new NotFoundError("OIDC provider not found."));

        if (!IsValidRedirectUri(command.RedirectUri))
            return Result.Failure<OidcLoginStartResult>(new BadRequestError("Redirect URI is invalid."));

        var returnUrl = OidcLoginUrl.NormalizeReturnUrl(command.ReturnUrl, command.RedirectUri, command.AllowedReturnOrigins);
        if (returnUrl is null)
            return Result.Failure<OidcLoginStartResult>(new BadRequestError("Return URL is invalid."));

        var discovery = await discoveryService.GetDiscoveryAsync(provider.Issuer, cancellationToken);
        if (!discovery.IsSuccess(out var discoveryResult))
            return Result.Failure<OidcLoginStartResult>(discovery.Errors);

        var state = CreateOpaqueValue(32);
        var nonce = CreateOpaqueValue(32);
        var codeVerifier = CreateOpaqueValue(64);
        var authorization = authenticationService.CreateAuthorizationRequest(
            provider,
            discoveryResult,
            command.RedirectUri,
            state,
            nonce,
            codeVerifier);

        var now = DateTime.UtcNow;
        await unitOfWork.OidcLoginStates.DeleteExpiredAsync(now, cancellationToken);
        await unitOfWork.OidcLoginStates.AddAsync(
            new OidcLoginState(
                Guid.CreateVersion7(),
                provider.Id,
                HashOpaqueValue(state),
                nonce,
                authorization.CodeVerifier,
                returnUrl,
                now,
                now.AddMinutes(10)),
            cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);

        return Result.Success(new OidcLoginStartResult(authorization.Url));
    }

    private static bool IsValidRedirectUri(string value)
        => Uri.TryCreate(value, UriKind.Absolute, out var uri)
           && uri.Scheme is "http" or "https";
}
