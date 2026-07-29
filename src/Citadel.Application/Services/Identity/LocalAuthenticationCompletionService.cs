using Application.Configs;
using Application.Features.Identity.Auth.Models;
using Application.Features.Identity.Mfa.Services;
using Application.Services;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Identity;
using Domain.Entities.Identity;
using Microsoft.Extensions.Options;

namespace Application.Services.Identity;

public interface ILocalAuthenticationCompletionService
{
    Task<LoginResponse> CompleteAsync(
        UserAuthInfo user,
        CancellationToken cancellationToken);
}

internal sealed class LocalAuthenticationCompletionService(
    IUnitOfWork unitOfWork,
    ITotpService totpService,
    IMfaPolicyService mfaPolicyService,
    ISecretValueProtector secretValueProtector,
    IAuthenticationSessionIssuer authenticationSessionIssuer,
    IMfaChallengeCookieService mfaChallengeCookieService,
    IMfaSetupCookieService mfaSetupCookieService,
    IOptions<MfaOptions> options) : ILocalAuthenticationCompletionService
{
    public async Task<LoginResponse> CompleteAsync(
        UserAuthInfo user,
        CancellationToken cancellationToken)
    {
        var now = DateTime.UtcNow;
        await unitOfWork.MfaChallenges.DeleteExpiredAsync(now, cancellationToken);
        await unitOfWork.UserMfa.DeleteExpiredSetupSessionsAsync(now, cancellationToken);

        var settings = await unitOfWork.UserMfa.GetSettingsAsync(user.Id, cancellationToken);
        if (settings is null && mfaPolicyService.RequiresMfa(user))
        {
            var expiresAt = now.AddMinutes(options.Value.SetupLifetimeMinutes);
            var setup = totpService.CreateSetup("Citadel", GetAccountName(user), expiresAt);
            var session = new MfaSetupSession(
                Guid.CreateVersion7(),
                user.Id,
                secretValueProtector.Protect(setup.Secret),
                expiresAt,
                null,
                now);

            await unitOfWork.UserMfa.DeleteSetupSessionsAsync(user.Id, cancellationToken);
            await unitOfWork.UserMfa.AddSetupSessionAsync(session, cancellationToken);
            await unitOfWork.CommitAsync(cancellationToken);

            mfaSetupCookieService.Set(session.Id, session.ExpiresAt);
            return new LoginResponse(null, LoginNextStep.EnrollMfa);
        }

        if (settings is not null)
        {
            var challenge = new MfaChallenge(
                Guid.CreateVersion7(),
                user.Id,
                now.AddMinutes(options.Value.ChallengeLifetimeMinutes),
                0,
                null,
                now);

            await unitOfWork.MfaChallenges.DeleteForUserAsync(user.Id, cancellationToken);
            await unitOfWork.MfaChallenges.AddAsync(challenge, cancellationToken);
            await unitOfWork.CommitAsync(cancellationToken);

            mfaChallengeCookieService.Set(challenge.Id, challenge.ExpiresAt);
            return new LoginResponse(null, LoginNextStep.VerifyMfa);
        }

        var accessToken = await authenticationSessionIssuer.IssueAsync(user, cancellationToken);
        return new LoginResponse(accessToken, LoginNextStep.Completed);
    }

    private static string GetAccountName(UserAuthInfo user)
        => string.IsNullOrWhiteSpace(user.Email) ? user.Name : user.Email;
}
