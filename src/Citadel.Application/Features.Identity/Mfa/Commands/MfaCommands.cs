using Application.Configs;
using Application.Features.Identity.Mfa.Models;
using Application.Features.Identity.Mfa.Services;
using Application.Services;
using Application.Services.Identity;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Identity;
using Domain.Entities.Activities;
using Domain.Entities.Identity;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.Attributes;
using Hosting.Common.Abstraction;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;
using Microsoft.Extensions.Logging;
using Microsoft.Extensions.Options;
using OtpNet;
using static Application.Features.Identity.Mfa.Commands.MfaCommandHelpers;

namespace Application.Features.Identity.Mfa.Commands;

public sealed record GetMfaStatus : IQuery<Result<MfaStatusResult>>;

public sealed record StartProfileMfaSetup(string Password) : ICommand<Result<MfaSetupResult>>
{
    internal sealed class Validator : AbstractValidator<StartProfileMfaSetup>
    {
        public Validator() => RuleFor(x => x.Password).NotNull().MinimumLength(6).MaximumLength(128);
    }
}

public sealed record ConfirmProfileMfaSetup(string Code) : ICommand<Result<MfaRecoveryCodesResult>>
{
    internal sealed class Validator : AbstractValidator<ConfirmProfileMfaSetup>
    {
        public Validator() => RuleFor(x => x.Code).Matches(@"^\d{6}$");
    }
}

public sealed record DisableProfileMfa(string Password, string? Code, string? RecoveryCode) : ICommand<Result>
{
    internal sealed class Validator : AbstractValidator<DisableProfileMfa>
    {
        public Validator()
        {
            RuleFor(x => x.Password).NotNull().MinimumLength(6).MaximumLength(128);
            RuleFor(x => x).Must(x => HasExactlyOne(x.Code, x.RecoveryCode))
                .WithMessage("Exactly one of Code or RecoveryCode is required.");
            When(x => !string.IsNullOrWhiteSpace(x.Code), () => RuleFor(x => x.Code!).Matches(@"^\d{6}$"));
            When(x => !string.IsNullOrWhiteSpace(x.RecoveryCode), () => RuleFor(x => x.RecoveryCode!).MaximumLength(64));
        }
    }
}

public sealed record RegenerateProfileMfaRecoveryCodes(string Password, string Code) : ICommand<Result<MfaRecoveryCodesResult>>
{
    internal sealed class Validator : AbstractValidator<RegenerateProfileMfaRecoveryCodes>
    {
        public Validator()
        {
            RuleFor(x => x.Password).NotNull().MinimumLength(6).MaximumLength(128);
            RuleFor(x => x.Code).Matches(@"^\d{6}$");
        }
    }
}

public sealed record GetMandatoryMfaSetup : IQuery<Result<MandatoryMfaSetupResult>>;

public sealed record ConfirmMandatoryMfaSetup(string Code) : ICommand<Result<MandatoryMfaSetupCompleteResult>>
{
    internal sealed class Validator : AbstractValidator<ConfirmMandatoryMfaSetup>
    {
        public Validator() => RuleFor(x => x.Code).Matches(@"^\d{6}$");
    }
}

public sealed record VerifyMfaChallenge(string? Code, string? RecoveryCode) : ICommand<Result<MfaVerificationResult>>
{
    internal sealed class Validator : AbstractValidator<VerifyMfaChallenge>
    {
        public Validator()
        {
            RuleFor(x => x).Must(x => HasExactlyOne(x.Code, x.RecoveryCode))
                .WithMessage("Exactly one of Code or RecoveryCode is required.");
            When(x => !string.IsNullOrWhiteSpace(x.Code), () => RuleFor(x => x.Code!).Matches(@"^\d{6}$"));
            When(x => !string.IsNullOrWhiteSpace(x.RecoveryCode), () => RuleFor(x => x.RecoveryCode!).MaximumLength(64));
        }
    }
}

[RequirePermission(ResourceType.User, PermissionLevel.Write, ResourceIdProperty = nameof(ResetUserMfa.UserId))]
public sealed record ResetUserMfa(Guid UserId) : ICommand<Result>, IAdministratorRequest;

internal sealed class GetMfaStatusHandler(
    IUnitOfWork unitOfWork,
    IUserContextAccessor userContext,
    IMfaPolicyService policyService)
    : IQueryHandler<GetMfaStatus, Result<MfaStatusResult>>
{
    public async ValueTask<Result<MfaStatusResult>> Handle(GetMfaStatus query, CancellationToken cancellationToken)
    {
        var authInfo = await GetCurrentAuthInfo(unitOfWork, userContext, cancellationToken);
        if (!authInfo.IsSuccess(out var user))
            return Result.Failure<MfaStatusResult>(authInfo.Errors);

        var settings = await unitOfWork.UserMfa.GetSettingsAsync(user.Id, cancellationToken);
        var remainingCodes = settings is null
            ? 0
            : await unitOfWork.UserMfa.CountUnusedRecoveryCodesAsync(user.Id, cancellationToken);

        return Result.Success(new MfaStatusResult(
            settings is not null,
            remainingCodes,
            policyService.CurrentPolicy,
            policyService.CanDisable(user)));
    }
}

internal sealed class StartProfileMfaSetupHandler(
    IUnitOfWork unitOfWork,
    IUserContextAccessor userContext,
    ITotpService totpService,
    ISecretValueProtector secretValueProtector,
    ICitadelPasswordHasher passwordHasher,
    IOptions<MfaOptions> options)
    : ICommandHandler<StartProfileMfaSetup, Result<MfaSetupResult>>
{
    public async ValueTask<Result<MfaSetupResult>> Handle(StartProfileMfaSetup command, CancellationToken cancellationToken)
    {
        var authInfo = await GetCurrentLocalAuthInfo(unitOfWork, userContext, cancellationToken);
        if (!authInfo.IsSuccess(out var user))
            return Result.Failure<MfaSetupResult>(authInfo.Errors);

        if (!passwordHasher.Verify(command.Password, user.Password!))
            return Result.Failure<MfaSetupResult>(new BadRequestError("Current password is incorrect."));

        if (await unitOfWork.UserMfa.GetSettingsAsync(user.Id, cancellationToken) is not null)
            return Result.Failure<MfaSetupResult>(new ConflictError("Two-factor authentication is already enabled."));

        var now = DateTime.UtcNow;
        var expiresAt = now.AddMinutes(options.Value.SetupLifetimeMinutes);
        var setup = totpService.CreateSetup("Citadel", GetAccountName(user), expiresAt);
        var session = new MfaSetupSession(
            Guid.CreateVersion7(),
            user.Id,
            secretValueProtector.Protect(setup.Secret),
            expiresAt,
            null,
            now);

        await unitOfWork.UserMfa.DeleteExpiredSetupSessionsAsync(now, cancellationToken);
        await unitOfWork.UserMfa.DeleteSetupSessionsAsync(user.Id, cancellationToken);
        await unitOfWork.UserMfa.AddSetupSessionAsync(session, cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);

        return Result.Success(new MfaSetupResult(setup.Secret, setup.OtpAuthUri, setup.ExpiresAt));
    }
}

internal sealed class ConfirmProfileMfaSetupHandler(
    IUnitOfWork unitOfWork,
    IUserContextAccessor userContext,
    ITotpService totpService,
    ISecretValueProtector secretValueProtector,
    IRecoveryCodeService recoveryCodeService,
    ICurrentRefreshSessionResolver currentRefreshSessionResolver,
    IOptions<MfaOptions> options,
    ILogger<ConfirmProfileMfaSetupHandler> logger)
    : ICommandHandler<ConfirmProfileMfaSetup, Result<MfaRecoveryCodesResult>>
{
    public async ValueTask<Result<MfaRecoveryCodesResult>> Handle(ConfirmProfileMfaSetup command, CancellationToken cancellationToken)
    {
        var authInfo = await GetCurrentLocalAuthInfo(unitOfWork, userContext, cancellationToken);
        if (!authInfo.IsSuccess(out var user))
            return Result.Failure<MfaRecoveryCodesResult>(authInfo.Errors);

        var now = DateTime.UtcNow;
        var session = await unitOfWork.UserMfa.GetActiveSetupSessionByUserAsync(user.Id, now, cancellationToken);
        if (session is null)
            return Result.Failure<MfaRecoveryCodesResult>(new BadRequestError("Invalid or expired setup session."));

        var secret = UnprotectSecret(secretValueProtector, session.ProtectedTotpSecret, logger);
        if (secret is null || !VerifyTotp(totpService, secret, command.Code, out var matchedTimeStep))
            return Result.Failure<MfaRecoveryCodesResult>(new BadRequestError("Invalid or expired verification code."));

        var recoveryCodes = CreateRecoveryCodes(user.Id, recoveryCodeService, options.Value.RecoveryCodeCount);
        if (await unitOfWork.UserMfa.TryConsumeSetupSessionAsync(session.Id, now, cancellationToken) != 1)
            return Result.Failure<MfaRecoveryCodesResult>(new BadRequestError("Invalid or expired setup session."));

        await unitOfWork.UserMfa.UpsertSettingsAsync(
            new UserMfaSettings(user.Id, session.ProtectedTotpSecret, matchedTimeStep, now, now),
            cancellationToken);
        await unitOfWork.UserMfa.ReplaceRecoveryCodesAsync(user.Id, recoveryCodes.HashedCodes, cancellationToken);
        await RevokeOtherSessionsAsync(unitOfWork, currentRefreshSessionResolver, user.Id, cancellationToken);
        await AddUserActivityAsync(unitOfWork, user, user.ActorId, ActivityEventType.UserMfaEnabled, new UserMfaEnabled(), cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);

        return Result.Success(new MfaRecoveryCodesResult(true, recoveryCodes.PlaintextCodes));
    }
}

internal sealed class DisableProfileMfaHandler(
    IUnitOfWork unitOfWork,
    IUserContextAccessor userContext,
    ITotpService totpService,
    ISecretValueProtector secretValueProtector,
    IRecoveryCodeService recoveryCodeService,
    IMfaPolicyService policyService,
    ICurrentRefreshSessionResolver currentRefreshSessionResolver,
    ICitadelPasswordHasher passwordHasher,
    ILogger<DisableProfileMfaHandler> logger)
    : ICommandHandler<DisableProfileMfa, Result>
{
    public async ValueTask<Result> Handle(DisableProfileMfa command, CancellationToken cancellationToken)
    {
        var authInfo = await GetCurrentLocalAuthInfo(unitOfWork, userContext, cancellationToken);
        if (!authInfo.IsSuccess(out var user))
            return Result.Failure(authInfo.Errors);

        if (!policyService.CanDisable(user))
            return Result.Failure(new BadRequestError("Two-factor authentication is required by policy."));

        if (!passwordHasher.Verify(command.Password, user.Password!))
            return Result.Failure(new BadRequestError("Current password is incorrect."));

        var settings = await unitOfWork.UserMfa.GetSettingsAsync(user.Id, cancellationToken);
        if (settings is null)
            return Result.Failure(new NotFoundError("Two-factor authentication is not enabled."));

        var now = DateTime.UtcNow;
        if (!await VerifyMfaCredentialAsync(unitOfWork, totpService, secretValueProtector, recoveryCodeService, settings, command.Code, command.RecoveryCode, now, logger, cancellationToken))
            return Result.Failure(new BadRequestError("Invalid or expired verification code."));

        await DeleteMfaStateAsync(unitOfWork, user.Id, cancellationToken);
        await RevokeOtherSessionsAsync(unitOfWork, currentRefreshSessionResolver, user.Id, cancellationToken);
        await AddUserActivityAsync(unitOfWork, user, user.ActorId, ActivityEventType.UserMfaDisabled, new UserMfaDisabled(), cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);

        return Result.Success();
    }
}

internal sealed class RegenerateProfileMfaRecoveryCodesHandler(
    IUnitOfWork unitOfWork,
    IUserContextAccessor userContext,
    ITotpService totpService,
    ISecretValueProtector secretValueProtector,
    IRecoveryCodeService recoveryCodeService,
    IOptions<MfaOptions> options,
    ICitadelPasswordHasher passwordHasher,
    ILogger<RegenerateProfileMfaRecoveryCodesHandler> logger)
    : ICommandHandler<RegenerateProfileMfaRecoveryCodes, Result<MfaRecoveryCodesResult>>
{
    public async ValueTask<Result<MfaRecoveryCodesResult>> Handle(RegenerateProfileMfaRecoveryCodes command, CancellationToken cancellationToken)
    {
        var authInfo = await GetCurrentLocalAuthInfo(unitOfWork, userContext, cancellationToken);
        if (!authInfo.IsSuccess(out var user))
            return Result.Failure<MfaRecoveryCodesResult>(authInfo.Errors);

        if (!passwordHasher.Verify(command.Password, user.Password!))
            return Result.Failure<MfaRecoveryCodesResult>(new BadRequestError("Current password is incorrect."));

        var settings = await unitOfWork.UserMfa.GetSettingsAsync(user.Id, cancellationToken);
        if (settings is null)
            return Result.Failure<MfaRecoveryCodesResult>(new NotFoundError("Two-factor authentication is not enabled."));

        var secret = UnprotectSecret(secretValueProtector, settings.ProtectedTotpSecret, logger);
        if (secret is null || !VerifyTotp(totpService, secret, command.Code, out var matchedTimeStep))
            return Result.Failure<MfaRecoveryCodesResult>(new BadRequestError("Invalid or expired verification code."));

        if (await unitOfWork.UserMfa.TryAcceptTimeStepAsync(user.Id, matchedTimeStep, cancellationToken) != 1)
            return Result.Failure<MfaRecoveryCodesResult>(new BadRequestError("Invalid or expired verification code."));

        var recoveryCodes = CreateRecoveryCodes(user.Id, recoveryCodeService, options.Value.RecoveryCodeCount);
        await unitOfWork.UserMfa.ReplaceRecoveryCodesAsync(user.Id, recoveryCodes.HashedCodes, cancellationToken);
        await AddUserActivityAsync(
            unitOfWork,
            user,
            user.ActorId,
            ActivityEventType.UserMfaRecoveryCodesRegenerated,
            new UserMfaRecoveryCodesRegenerated(),
            cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);

        return Result.Success(new MfaRecoveryCodesResult(true, recoveryCodes.PlaintextCodes));
    }
}

internal sealed class GetMandatoryMfaSetupHandler(
    IUnitOfWork unitOfWork,
    IMfaSetupCookieService setupCookieService,
    ITotpService totpService,
    ISecretValueProtector secretValueProtector,
    ILogger<GetMandatoryMfaSetupHandler> logger)
    : IQueryHandler<GetMandatoryMfaSetup, Result<MandatoryMfaSetupResult>>
{
    public async ValueTask<Result<MandatoryMfaSetupResult>> Handle(GetMandatoryMfaSetup query, CancellationToken cancellationToken)
    {
        var sessionId = setupCookieService.GetCurrent();
        if (!sessionId.HasValue)
            return Result.Failure<MandatoryMfaSetupResult>(new UnauthorizedError("Invalid or expired setup session."));

        var session = await unitOfWork.UserMfa.GetSetupSessionAsync(sessionId.Value, cancellationToken);
        if (session is null || !session.IsActive(DateTime.UtcNow))
            return Result.Failure<MandatoryMfaSetupResult>(new UnauthorizedError("Invalid or expired setup session."));

        var user = await unitOfWork.Users.GetUserAuthInfoByIdAsync(session.UserId, cancellationToken);
        if (user is null)
            return Result.Failure<MandatoryMfaSetupResult>(new UnauthorizedError("Invalid or expired setup session."));

        var secret = UnprotectSecret(secretValueProtector, session.ProtectedTotpSecret, logger);
        if (secret is null)
            return Result.Failure<MandatoryMfaSetupResult>(new UnauthorizedError("Invalid or expired setup session."));

        return Result.Success(new MandatoryMfaSetupResult(
            secret,
            totpService.BuildOtpAuthUri("Citadel", GetAccountName(user), secret),
            session.ExpiresAt));
    }
}

internal sealed class ConfirmMandatoryMfaSetupHandler(
    IUnitOfWork unitOfWork,
    IMfaSetupCookieService setupCookieService,
    ITotpService totpService,
    ISecretValueProtector secretValueProtector,
    IRecoveryCodeService recoveryCodeService,
    IAuthenticationSessionIssuer authenticationSessionIssuer,
    IOptions<MfaOptions> options,
    ILogger<ConfirmMandatoryMfaSetupHandler> logger)
    : ICommandHandler<ConfirmMandatoryMfaSetup, Result<MandatoryMfaSetupCompleteResult>>
{
    public async ValueTask<Result<MandatoryMfaSetupCompleteResult>> Handle(ConfirmMandatoryMfaSetup command, CancellationToken cancellationToken)
    {
        var sessionId = setupCookieService.GetCurrent();
        if (!sessionId.HasValue)
            return Result.Failure<MandatoryMfaSetupCompleteResult>(new UnauthorizedError("Invalid or expired setup session."));

        var now = DateTime.UtcNow;
        var session = await unitOfWork.UserMfa.GetSetupSessionAsync(sessionId.Value, cancellationToken);
        if (session is null || !session.IsActive(now))
            return Result.Failure<MandatoryMfaSetupCompleteResult>(new UnauthorizedError("Invalid or expired setup session."));

        var user = await unitOfWork.Users.GetUserAuthInfoByIdAsync(session.UserId, cancellationToken);
        if (user is null)
            return Result.Failure<MandatoryMfaSetupCompleteResult>(new UnauthorizedError("Invalid or expired setup session."));

        var secret = UnprotectSecret(secretValueProtector, session.ProtectedTotpSecret, logger);
        if (secret is null || !VerifyTotp(totpService, secret, command.Code, out var matchedTimeStep))
            return Result.Failure<MandatoryMfaSetupCompleteResult>(new BadRequestError("Invalid or expired verification code."));

        var recoveryCodes = CreateRecoveryCodes(user.Id, recoveryCodeService, options.Value.RecoveryCodeCount);
        if (await unitOfWork.UserMfa.TryConsumeSetupSessionAsync(session.Id, now, cancellationToken) != 1)
            return Result.Failure<MandatoryMfaSetupCompleteResult>(new UnauthorizedError("Invalid or expired setup session."));

        await unitOfWork.UserMfa.UpsertSettingsAsync(
            new UserMfaSettings(user.Id, session.ProtectedTotpSecret, matchedTimeStep, now, now),
            cancellationToken);
        await unitOfWork.UserMfa.ReplaceRecoveryCodesAsync(user.Id, recoveryCodes.HashedCodes, cancellationToken);
        await AddUserActivityAsync(unitOfWork, user, user.ActorId, ActivityEventType.UserMfaEnabled, new UserMfaEnabled(), cancellationToken);

        var accessToken = await authenticationSessionIssuer.IssueAsync(user, cancellationToken);
        setupCookieService.Delete();

        return Result.Success(new MandatoryMfaSetupCompleteResult(accessToken, recoveryCodes.PlaintextCodes));
    }
}

internal sealed class VerifyMfaChallengeHandler(
    IUnitOfWork unitOfWork,
    IMfaChallengeCookieService challengeCookieService,
    ITotpService totpService,
    ISecretValueProtector secretValueProtector,
    IRecoveryCodeService recoveryCodeService,
    IAuthenticationSessionIssuer authenticationSessionIssuer,
    IOptions<MfaOptions> options,
    ILogger<VerifyMfaChallengeHandler> logger)
    : ICommandHandler<VerifyMfaChallenge, Result<MfaVerificationResult>>
{
    public async ValueTask<Result<MfaVerificationResult>> Handle(VerifyMfaChallenge command, CancellationToken cancellationToken)
    {
        var invalid = new BadRequestError("Invalid or expired verification code.");
        var challengeId = challengeCookieService.GetCurrent();
        if (!challengeId.HasValue)
            return Result.Failure<MfaVerificationResult>(invalid);

        var now = DateTime.UtcNow;
        var challenge = await unitOfWork.MfaChallenges.GetAsync(challengeId.Value, cancellationToken);
        if (challenge is null || !challenge.IsActive(now, options.Value.MaxFailedAttempts))
            return Result.Failure<MfaVerificationResult>(invalid);

        var user = await unitOfWork.Users.GetUserAuthInfoByIdAsync(challenge.UserId, cancellationToken);
        var settings = await unitOfWork.UserMfa.GetSettingsAsync(challenge.UserId, cancellationToken);
        if (user is null || settings is null)
            return Result.Failure<MfaVerificationResult>(invalid);

        var verified = false;
        var usedRecoveryCode = false;
        if (!string.IsNullOrWhiteSpace(command.Code))
        {
            var secret = UnprotectSecret(secretValueProtector, settings.ProtectedTotpSecret, logger);
            if (secret is not null && VerifyTotp(totpService, secret, command.Code, out var matchedTimeStep))
                verified = await unitOfWork.UserMfa.TryAcceptTimeStepAsync(user.Id, matchedTimeStep, cancellationToken) == 1;
        }
        else if (!string.IsNullOrWhiteSpace(command.RecoveryCode))
        {
            var hash = recoveryCodeService.Hash(recoveryCodeService.Normalize(command.RecoveryCode));
            verified = await unitOfWork.UserMfa.TryUseRecoveryCodeAsync(user.Id, hash, now, cancellationToken) == 1;
            usedRecoveryCode = verified;
        }

        if (!verified)
        {
            await unitOfWork.MfaChallenges.TryIncrementFailedAttemptsAsync(challenge.Id, now, options.Value.MaxFailedAttempts, cancellationToken);
            await AddUserActivityAsync(unitOfWork, user, user.ActorId, ActivityEventType.UserMfaVerificationFailed, new UserMfaVerificationFailed(), cancellationToken);
            await unitOfWork.CommitAsync(cancellationToken);
            return Result.Failure<MfaVerificationResult>(invalid);
        }

        if (await unitOfWork.MfaChallenges.TryConsumeAsync(challenge.Id, now, options.Value.MaxFailedAttempts, cancellationToken) != 1)
        {
            await unitOfWork.RollbackAsync();
            return Result.Failure<MfaVerificationResult>(invalid);
        }

        if (usedRecoveryCode)
            await AddUserActivityAsync(unitOfWork, user, user.ActorId, ActivityEventType.UserMfaRecoveryCodeUsed, new UserMfaRecoveryCodeUsed(), cancellationToken);

        var accessToken = await authenticationSessionIssuer.IssueAsync(user, cancellationToken);
        challengeCookieService.Delete();

        return Result.Success(new MfaVerificationResult(accessToken));
    }
}

internal sealed class ResetUserMfaHandler(
    IUnitOfWork unitOfWork,
    IUserContextAccessor userContext)
    : ICommandHandler<ResetUserMfa, Result>
{
    public async ValueTask<Result> Handle(ResetUserMfa command, CancellationToken cancellationToken)
    {
        var actorId = userContext.Current.ActorId;
        if (actorId == Guid.Empty)
            return Result.Failure(new UnauthorizedError("Missing user context"));

        var user = await unitOfWork.Users.GetUserAuthInfoByIdAsync(command.UserId, cancellationToken);
        if (user is null)
            return Result.Failure(new NotFoundError("The provided user does not exist."));

        await DeleteMfaStateAsync(unitOfWork, user.Id, cancellationToken);
        await unitOfWork.RefreshTokens.DeleteAllTokensAsync(user.Id, cancellationToken);
        await AddUserActivityAsync(
            unitOfWork,
            user,
            actorId,
            ActivityEventType.UserMfaResetByAdministrator,
            new UserMfaResetByAdministrator(user.Id),
            cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);

        return Result.Success();
    }
}

internal static class MfaCommandHelpers
{
    internal static bool HasExactlyOne(string? first, string? second)
        => string.IsNullOrWhiteSpace(first) != string.IsNullOrWhiteSpace(second);

    internal static async Task<Result<UserAuthInfo>> GetCurrentAuthInfo(
        IUnitOfWork unitOfWork,
        IUserContextAccessor userContext,
        CancellationToken cancellationToken)
    {
        var userId = userContext.Current.UserId;
        if (userId == Guid.Empty)
            return Result.Failure<UserAuthInfo>(new UnauthorizedError("Missing user context"));

        var authInfo = await unitOfWork.Users.GetUserAuthInfoByIdAsync(userId, cancellationToken);
        return authInfo is null
            ? Result.Failure<UserAuthInfo>(new NotFoundError("Current user does not exist"))
            : Result.Success(authInfo);
    }

    internal static async Task<Result<UserAuthInfo>> GetCurrentLocalAuthInfo(
        IUnitOfWork unitOfWork,
        IUserContextAccessor userContext,
        CancellationToken cancellationToken)
    {
        var authInfo = await GetCurrentAuthInfo(unitOfWork, userContext, cancellationToken);
        if (!authInfo.IsSuccess(out var user))
            return authInfo;

        return string.IsNullOrWhiteSpace(user.Password)
            ? Result.Failure<UserAuthInfo>(new BadRequestError("This account does not have a local password credential."))
            : Result.Success(user);
    }

    internal static async Task<bool> VerifyMfaCredentialAsync(
        IUnitOfWork unitOfWork,
        ITotpService totpService,
        ISecretValueProtector secretValueProtector,
        IRecoveryCodeService recoveryCodeService,
        UserMfaSettings settings,
        string? code,
        string? recoveryCode,
        DateTime now,
        ILogger logger,
        CancellationToken cancellationToken)
    {
        if (!string.IsNullOrWhiteSpace(code))
        {
            var secret = UnprotectSecret(secretValueProtector, settings.ProtectedTotpSecret, logger);
            return secret is not null
                   && VerifyTotp(totpService, secret, code, out var matchedTimeStep)
                   && await unitOfWork.UserMfa.TryAcceptTimeStepAsync(settings.UserId, matchedTimeStep, cancellationToken) == 1;
        }

        if (string.IsNullOrWhiteSpace(recoveryCode))
            return false;

        var hash = recoveryCodeService.Hash(recoveryCodeService.Normalize(recoveryCode));
        return await unitOfWork.UserMfa.TryUseRecoveryCodeAsync(settings.UserId, hash, now, cancellationToken) == 1;
    }

    internal static bool VerifyTotp(ITotpService totpService, string secret, string code, out long matchedTimeStep)
        => totpService.TryVerify(Base32Encoding.ToBytes(secret), code, out matchedTimeStep);

    internal static string? UnprotectSecret(ISecretValueProtector secretValueProtector, string protectedSecret, ILogger logger)
    {
        try
        {
            return secretValueProtector.Unprotect(protectedSecret);
        }
        catch (Exception ex)
        {
            logger.LogWarning(ex, "Failed to unprotect MFA TOTP secret.");
            return null;
        }
    }

    internal static (IReadOnlyList<string> PlaintextCodes, IReadOnlyCollection<UserMfaRecoveryCode> HashedCodes) CreateRecoveryCodes(
        Guid userId,
        IRecoveryCodeService recoveryCodeService,
        int count)
    {
        var plaintext = recoveryCodeService.Generate(count);
        var hashed = plaintext
            .Select(code => new UserMfaRecoveryCode(
                Guid.CreateVersion7(),
                userId,
                recoveryCodeService.Hash(recoveryCodeService.Normalize(code)),
                null,
                DateTime.UtcNow))
            .ToArray();

        return (plaintext, hashed);
    }

    internal static async Task DeleteMfaStateAsync(IUnitOfWork unitOfWork, Guid userId, CancellationToken cancellationToken)
    {
        await unitOfWork.UserMfa.DeleteSettingsAsync(userId, cancellationToken);
        await unitOfWork.UserMfa.DeleteRecoveryCodesAsync(userId, cancellationToken);
        await unitOfWork.UserMfa.DeleteSetupSessionsAsync(userId, cancellationToken);
        await unitOfWork.MfaChallenges.DeleteForUserAsync(userId, cancellationToken);
    }

    internal static async Task RevokeOtherSessionsAsync(
        IUnitOfWork unitOfWork,
        ICurrentRefreshSessionResolver currentRefreshSessionResolver,
        Guid userId,
        CancellationToken cancellationToken)
    {
        var currentSessionId = await currentRefreshSessionResolver.ResolveAsync(userId, unitOfWork, cancellationToken);
        if (currentSessionId.HasValue)
            await unitOfWork.RefreshTokens.DeleteOtherTokensAsync(userId, currentSessionId.Value, cancellationToken);
        else
            await unitOfWork.RefreshTokens.DeleteAllTokensAsync(userId, cancellationToken);
    }

    internal static Task AddUserActivityAsync(
        IUnitOfWork unitOfWork,
        UserAuthInfo user,
        Guid actorId,
        ActivityEventType eventType,
        ActivityEventInfo info,
        CancellationToken cancellationToken)
        => unitOfWork.ActivityEventRepository.AddAsync(
            new ActivityEvent(
                platformId: null,
                resourceId: user.Id,
                actorId: actorId,
                resourceName: user.Name,
                eventType: eventType,
                status: ActivityStatus.Success,
                info: info),
            cancellationToken);

    internal static string GetAccountName(UserAuthInfo user)
        => string.IsNullOrWhiteSpace(user.Email) ? user.Name : user.Email;
}
