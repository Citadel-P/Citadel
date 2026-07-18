using Application.Configs;
using Application.Features.Identity.Auth.Models;
using Application.Features.Identity.Mfa.Services;
using Application.Services;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Identity;
using Domain.Entities.Identity;
using FluentValidation;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;
using Microsoft.Extensions.Options;

namespace Application.Features.Identity.Auth.Commands;

public sealed record LoginCommand(string EmailOrName, string Password) : ICommand<Result<LoginResponse>>
{
    internal class Validator : AbstractValidator<LoginCommand>
    {
        public Validator()
        {
            RuleFor(x => x.EmailOrName).NotEmpty();
            RuleFor(x => x.Password).NotNull().MinimumLength(6).MaximumLength(128);
        }
    }
}

internal sealed class LoginCommandHandler(
    IUnitOfWork unitOfWork,
    ITotpService totpService,
    IMfaPolicyService mfaPolicyService,
    ISecretValueProtector secretValueProtector,
    IAuthenticationSessionIssuer authenticationSessionIssuer,
    IMfaChallengeCookieService mfaChallengeCookieService,
    IMfaSetupCookieService mfaSetupCookieService,
    IOptions<MfaOptions> options) : ICommandHandler<LoginCommand, Result<LoginResponse>>
{
    public async ValueTask<Result<LoginResponse>> Handle(LoginCommand query, CancellationToken cancellationToken)
    {
        var userAuthInfo = await unitOfWork.Users.GetUserAuthInfoByEmailOrNameAsync(query.EmailOrName, cancellationToken);
        if (userAuthInfo is null)
            return Result.Failure<LoginResponse>(new NotFoundError("Invalid credentials"));

        if (!User.IsValidPassword(query.Password, userAuthInfo.Password ?? string.Empty))
            return Result.Failure<LoginResponse>(new BadRequestError("Invalid credentials"));

        var now = DateTime.UtcNow;
        await unitOfWork.MfaChallenges.DeleteExpiredAsync(now, cancellationToken);
        await unitOfWork.UserMfa.DeleteExpiredSetupSessionsAsync(now, cancellationToken);

        var settings = await unitOfWork.UserMfa.GetSettingsAsync(userAuthInfo.Id, cancellationToken);
        if (settings is null && mfaPolicyService.RequiresMfa(userAuthInfo))
        {
            var expiresAt = now.AddMinutes(options.Value.SetupLifetimeMinutes);
            var setup = totpService.CreateSetup("Citadel", GetAccountName(userAuthInfo), expiresAt);
            var session = new MfaSetupSession(
                Guid.CreateVersion7(),
                userAuthInfo.Id,
                secretValueProtector.Protect(setup.Secret),
                expiresAt,
                null,
                now);

            await unitOfWork.UserMfa.DeleteSetupSessionsAsync(userAuthInfo.Id, cancellationToken);
            await unitOfWork.UserMfa.AddSetupSessionAsync(session, cancellationToken);
            await unitOfWork.CommitAsync(cancellationToken);

            mfaSetupCookieService.Set(session.Id, session.ExpiresAt);
            return Result.Success(new LoginResponse(null, LoginNextStep.EnrollMfa));
        }

        if (settings is not null)
        {
            var challenge = new MfaChallenge(
                Guid.CreateVersion7(),
                userAuthInfo.Id,
                now.AddMinutes(options.Value.ChallengeLifetimeMinutes),
                0,
                null,
                now);

            await unitOfWork.MfaChallenges.DeleteForUserAsync(userAuthInfo.Id, cancellationToken);
            await unitOfWork.MfaChallenges.AddAsync(challenge, cancellationToken);
            await unitOfWork.CommitAsync(cancellationToken);

            mfaChallengeCookieService.Set(challenge.Id, challenge.ExpiresAt);
            return Result.Success(new LoginResponse(null, LoginNextStep.VerifyMfa));
        }

        var accessToken = await authenticationSessionIssuer.IssueAsync(userAuthInfo, cancellationToken);
        return Result.Success(new LoginResponse(accessToken, LoginNextStep.Completed));
    }

    private static string GetAccountName(UserAuthInfo user)
        => string.IsNullOrWhiteSpace(user.Email) ? user.Name : user.Email;
}
