using Application.Services;
using Hosting.Common;
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
        var refreshToken = context.HttpContext?.Request.Cookies[Constants.RefreshToken];

        if (string.IsNullOrWhiteSpace(refreshToken))
            return Result.Failure<string>(new BadRequestError("Please include a refresh token in the request."));

        if (!jwtService.TryValidate(refreshToken, out var tokenId))
            return Result.Failure<string>(new UnauthorizedError("Invalid refresh token."));

        var existing = await dbContext.RefreshTokens.AsNoTracking().Include(s => s.User).ThenInclude(s => s.Teams).ThenInclude(s => s.Role)
           
            .FirstOrDefaultAsync(s => s.Id == tokenId, cancellationToken);
        
        if (existing == null)
            return Result.Failure<string>(new UnauthorizedError("Refresh token does not exist."));

        var accessToken = jwtService.CreateAccessToken(existing.User.GetJwtClaims());
        
        return Result.Success(accessToken);
    }
}