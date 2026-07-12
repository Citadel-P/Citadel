using Application.Services;
using Domain.Contracts.Interfaces;
using Domain.Entities.Identity;
using Hosting.Common;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.Identity.Auth.Commands;

public sealed record RefreshTokenCommand() : ICommand<Result<string>>;

internal sealed class RefreshTokenCommandHandler(
    IUnitOfWork unitOfWork,
    IJwtService jwtService,
    IRefreshTokenCookieService refreshTokenCookieService,
    IRequestSessionMetadataAccessor requestSessionMetadataAccessor)
    : ICommandHandler<RefreshTokenCommand, Result<string>>
{
    public async ValueTask<Result<string>> Handle(RefreshTokenCommand query, CancellationToken cancellationToken)
    {
        var refreshToken = refreshTokenCookieService.GetCurrent();

        if (string.IsNullOrWhiteSpace(refreshToken))
            return Result.Failure<string>(new UnauthorizedError("Missing refresh token."));

        if (!jwtService.TryValidate(refreshToken, out var tokenId))
            return Result.Failure<string>(new UnauthorizedError("Invalid refresh token."));

        var existing = await unitOfWork.RefreshTokens.GetUserAuthInfoByRefreshTokenIdAsync(tokenId, cancellationToken);
        if (existing == null)
            return Result.Failure<string>(new UnauthorizedError("Invalid refresh token."));

        var accessToken = jwtService.CreateAccessToken(User.GetJwtClaims(existing));
        var metadata = requestSessionMetadataAccessor.GetCurrent();
        await unitOfWork.RefreshTokens.TouchAsync(tokenId, DateTime.UtcNow, metadata.UserAgent, metadata.IpAddress, cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);
        
        return Result.Success(accessToken);
    }
}
