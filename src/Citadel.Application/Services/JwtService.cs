using System.IdentityModel.Tokens.Jwt;
using System.Security.Claims;
using System.Text;
using Application.Configs;
using Hosting.Common;
using Microsoft.Extensions.Options;
using Application.Services.Identity;
using Microsoft.IdentityModel.Tokens;

namespace Application.Services;

public interface IJwtService
{
    string CreateAccessToken(IEnumerable<Claim> claims);
    (Guid Id, string Token, DateTime ExpiresAt) CreateRefreshToken();
    bool TryValidate(string refreshToken, out Guid tokenId, out DateTime expiresAt);
}

internal sealed class JwtService(
    IOptions<JwtConfiguration> jwtConfig,
    IRoleCache roleCache) : IJwtService
{
    private readonly JwtConfiguration jwtConfig = jwtConfig.Value;

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
        var token = tokenHandler.CreateToken(tokenDescriptor);
        var written = tokenHandler.WriteToken(token);

        // Populate server-side role cache from claims 
        if (claims is not null)
        {
            var sub = claims.FirstOrDefault(c => c.Type == JwtRegisteredClaimNames.Sub)?.Value;
            if (Guid.TryParse(sub, out var userId))
            {
                var roles = claims.Where(c => c.Type == ClaimTypes.Role || string.Equals(c.Type, "role", StringComparison.OrdinalIgnoreCase))
                                  .Select(c => c.Value);
                roleCache.SetRoles(userId, roles);
            }
        }

        return written;
    }

    public (Guid Id, string Token, DateTime ExpiresAt) CreateRefreshToken()
    {
        var tokenId = Guid.CreateVersion7();
        var expiresAt = DateTime.UtcNow.AddDays(jwtConfig.RefreshToken.ValidForDays);

        string issuer = jwtConfig.Issuer ?? throw new ArgumentNullException(nameof(jwtConfig.Issuer));
        string audience = jwtConfig.Audience ?? throw new ArgumentNullException(nameof(jwtConfig.Audience));

        var tokenDescriptor = new SecurityTokenDescriptor
        {
            Subject = new ClaimsIdentity([new Claim(JwtRegisteredClaimNames.Jti, tokenId.ToString())]),
            Issuer = issuer,
            Audience = audience,
            Expires = expiresAt,
            SigningCredentials = GetSigningCredentials(),
        };

        var tokenHandler = new JwtSecurityTokenHandler();
        var refreshToken = tokenHandler.WriteToken(tokenHandler.CreateToken(tokenDescriptor));

        return (tokenId, refreshToken, expiresAt);
    }

    public bool TryValidate(string refreshToken, out Guid tokenId, out DateTime expiresAt)
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
            expiresAt = jwt.ValidTo;
            return valid;
        }
        catch (Exception)
        {
            tokenId = default;
            expiresAt = default;
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
            throw new ArgumentException("Secret key for algorithm: 'HS256' must be at least '256' bits long");
        }

        return new SigningCredentials(new SymmetricSecurityKey(key), SecurityAlgorithms.HmacSha256Signature);
    }
}
