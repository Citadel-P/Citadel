using Application.Services;
using Domain.Contracts.Interfaces;
using LightResults;
using Mediator;

namespace Application.Features.Identity.Auth.Commands;

public sealed record LogoutCommand(): ICommand<Result>;

internal sealed class LogoutCommandHandler(
    IUnitOfWork unitOfWork,
    IJwtService jwtService,
    IRefreshTokenCookieService refreshTokenCookieService,
    IMfaChallengeCookieService mfaChallengeCookieService,
    IMfaSetupCookieService mfaSetupCookieService)
    : ICommandHandler<LogoutCommand, Result>
{
    public async ValueTask<Result> Handle(LogoutCommand query, CancellationToken cancellationToken)
    {
        var refreshToken = refreshTokenCookieService.GetCurrent();
        mfaChallengeCookieService.Delete();
        mfaSetupCookieService.Delete();

        if (string.IsNullOrWhiteSpace(refreshToken))
            return Result.Success();

        refreshTokenCookieService.Delete();

        if (!jwtService.TryValidate(refreshToken, out var tokenId))
            return Result.Success();

        await unitOfWork.RefreshTokens.DeleteAsync(tokenId, cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);

        return Result.Success();
    }
}


