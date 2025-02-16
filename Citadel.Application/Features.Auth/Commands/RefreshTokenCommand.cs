using Application.Services;
using Application.Utils;
using Hosting.Common.ErrorTypes;
using Infrastructure.EntityFramework;
using LightResults;
using Mediator;
using Microsoft.AspNetCore.Http;
using Microsoft.EntityFrameworkCore;

namespace Application.Features.Auth.Commands;

public sealed record RefreshTokenCommand() : ICommand<Result<string>>;

internal sealed class RefreshTokenCommandHandler(
    IJwtService jwtService,
    IHttpContextAccessor context,
    ApplicationDbContext dbContext) : ICommandHandler<RefreshTokenCommand, Result<string>>
{
    public async ValueTask<Result<string>> Handle(RefreshTokenCommand query, CancellationToken cancellationToken)
    {
        var refreshToken = context.HttpContext.Request.Cookies[Constants.RefreshToken];

        if (string.IsNullOrWhiteSpace(refreshToken))
            return Result.Failure<string>(new BadRequestError("Please include a refresh token in the request."));

        if (!jwtService.TryValidate(refreshToken, out var tokenId))
            return Result.Failure<string>(new BadRequestError("Invalid refresh token."));

        var existing = await dbContext.RefreshTokens.Include(s => s.User).FirstOrDefaultAsync(s => s.Id == tokenId, cancellationToken);
        if (existing == null)
        {
            return Result.Failure<string>(new BadRequestError("Refresh token does not exist."));
        }

        var accessToken = jwtService.CreateAccessToken(existing.User.GetJwtClaims());
        
        return Result.Success(accessToken);
    }
}