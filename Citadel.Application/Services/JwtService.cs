using System.IdentityModel.Tokens.Jwt;
using System.Security.Claims;
using System.Text;
using Application.Configs;
using Hosting.Common;
using Microsoft.AspNetCore.Http;
using Microsoft.Extensions.Options;
using Microsoft.IdentityModel.Tokens;

namespace Application.Services;

public interface IJwtService
{
    string CreateAccessToken(IEnumerable<Claim> claims);
    (Guid, string) CreateRefreshToken();
    bool TryValidate(string refreshToken, out Guid tokenId);
}

internal sealed class JwtService(IHttpContextAccessor context, IOptions<JwtConfig> jwtConfig) : IJwtService
{
    private readonly JwtConfig jwtConfig = jwtConfig.Value;

    /// <inheritdoc />
    public string CreateAccessToken(IEnumerable<Claim> claims)
    {
        string issuer = jwtConfig.Issuer ?? throw new ArgumentNullException(nameof(jwtConfig.Issuer));
        string audience = jwtConfig.Audience ?? throw new ArgumentNullException(nameof(jwtConfig.Audience));

        var tokenDescriptor = new SecurityTokenDescriptor()
        {
            Subject = new ClaimsIdentity(claims ?? []),
            Issuer = issuer,
            Audience = audience,
            Expires = DateTime.UtcNow.AddMinutes(jwtConfig.AccessToken.ValidForMinutes),
            SigningCredentials = GetSigningCredentials(),
        };

        JwtSecurityTokenHandler tokenHandler = new();
        return tokenHandler.WriteToken(tokenHandler.CreateToken(tokenDescriptor));
    }

    public (Guid, string) CreateRefreshToken()
    {
        var tokenId = Guid.CreateVersion7();

        string issuer = jwtConfig.Issuer ?? throw new ArgumentNullException(nameof(jwtConfig.Issuer));
        string audience = jwtConfig.Audience ?? throw new ArgumentNullException(nameof(jwtConfig.Audience));

        var tokenDescriptor = new SecurityTokenDescriptor
        {
            Subject = new ClaimsIdentity([new Claim(JwtRegisteredClaimNames.Jti, tokenId.ToString())]),
            Issuer = issuer,
            Audience = audience,
            Expires = DateTime.UtcNow.AddDays(jwtConfig.RefreshToken.ValidForDays),
            SigningCredentials = GetSigningCredentials(),
        };

        var tokenHandler = new JwtSecurityTokenHandler();
        var refreshToken = tokenHandler.WriteToken(tokenHandler.CreateToken(tokenDescriptor));

        context.HttpContext.Response.Cookies.Append(Constants.RefreshToken, refreshToken, new CookieOptions
        {
            HttpOnly = true,
            Secure = true,
            IsEssential = true,
            SameSite = SameSiteMode.Strict,
            Expires = DateTime.Now.AddDays(jwtConfig.RefreshToken.ValidForDays),
            MaxAge = new TimeSpan(jwtConfig.RefreshToken.ValidForDays, 0, 0, 0)
        });

        return (tokenId, refreshToken);
    }

    public bool TryValidate(string refreshToken, out Guid tokenId)
    {
        string issuer = jwtConfig.Issuer ?? throw new ArgumentNullException(nameof(jwtConfig.Issuer));
        string audience = jwtConfig.Audience ?? throw new ArgumentNullException(nameof(jwtConfig.Audience));

        var tokenHandler = new JwtSecurityTokenHandler();
        var tokenValidationParams = new TokenValidationParameters
        {
            ValidateIssuerSigningKey = true,
            IssuerSigningKey = GetSigningCredentials().Key,
            ValidateIssuer = true,
            ValidateAudience = true,
            ClockSkew = TimeSpan.Zero,
            ValidAudience = audience,
            ValidIssuer = issuer,
        };

        try
        {
            tokenHandler.ValidateToken(refreshToken, tokenValidationParams, out SecurityToken token);
            var jwt = (JwtSecurityToken)token;
            var valid = Guid.TryParse(jwt.Id, out var id);
            tokenId = id;
            return valid;
        }
        catch (Exception)
        {
            tokenId = default;
            return false;
        }
    }

    private SigningCredentials GetSigningCredentials()
    {
        string jwtKey = string.IsNullOrEmpty(jwtConfig.Key)
                            ? Helpers.GetJwtSecretFromFile()
                            : jwtConfig.Key;
        byte[] key = Encoding.UTF8.GetBytes(jwtKey);
        if (key.Length < 32)
        {
            throw new ArgumentException("Secret key for algorithm: 'HS256' must be at least '256' bit long");
        }

        return new SigningCredentials(new SymmetricSecurityKey(key), SecurityAlgorithms.HmacSha256Signature);
    }
}