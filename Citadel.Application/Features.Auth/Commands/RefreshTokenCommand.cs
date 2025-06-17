using Application.Services;
using Domain.Contracts.Interfaces;
using Hosting.Common;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;
using Microsoft.AspNetCore.Http;
using Microsoft.EntityFrameworkCore;

namespace Application.Features.Auth.Commands;

public sealed record RefreshTokenCommand() : ICommand<Result<string>>;

internal sealed class RefreshTokenCommandHandler(IUnitOfWork unitOfWork, IJwtService jwtService, IHttpContextAccessor context) 
    : ICommandHandler<RefreshTokenCommand, Result<string>>
{
    public async ValueTask<Result<string>> Handle(RefreshTokenCommand query, CancellationToken cancellationToken)
    {
        var refreshToken = context.HttpContext?.Request.Cookies[Constants.RefreshToken];

        if (string.IsNullOrWhiteSpace(refreshToken))
            return Result.Failure<string>(new BadRequestError("Please include a refresh token in the request."));

        if (!jwtService.TryValidate(refreshToken, out var tokenId))
            return Result.Failure<string>(new UnauthorizedError("Invalid refresh token."));

        var existing = await unitOfWork.RefreshTokens.Query().AsNoTracking().Include(s => s.User).ThenInclude(s => s.Teams).ThenInclude(s => s.Role)
           
            .FirstOrDefaultAsync(s => s.Id == tokenId, cancellationToken);
        
        if (existing == null)
            return Result.Failure<string>(new UnauthorizedError("Refresh token does not exist."));

        var accessToken = jwtService.CreateAccessToken(existing.User.GetJwtClaims());
        
        return Result.Success(accessToken);
    }
}