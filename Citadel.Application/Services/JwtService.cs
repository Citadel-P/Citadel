using System.IdentityModel.Tokens.Jwt;
using System.Security.Claims;
using System.Text;
using Application.Configs;
using Application.Utils;
using Microsoft.Extensions.Options;
using Microsoft.IdentityModel.Tokens;

namespace Application.Services;

public interface IJwtService
{
    /// <summary>
    /// Create a Jwt token
    /// </summary>
    string CreateToken(IEnumerable<Claim> claims);
}

internal sealed class JwtService(IOptions<JwtConfig> jwtConfig) : IJwtService
{
    private readonly JwtConfig jwtConfig = jwtConfig.Value;

    /// <inheritdoc />
    public string CreateToken(IEnumerable<Claim> claims)
    {
        _ = claims ?? throw new ArgumentNullException();

        string jwtKey = string.IsNullOrEmpty(jwtConfig.Key)
                            ? Helpers.GetJwtSecretFromFile(Constants.JwtFilePath)
                            : jwtConfig.Key;

        byte[] key = Encoding.UTF8.GetBytes(jwtKey);
        if (key.Length < 32)
        {
            throw new ArgumentException("Secret key for algorithm: 'HS512' must be at least '256' bit long");
        }

        string issuer = jwtConfig.Issuer ?? throw new ArgumentNullException(nameof(jwtConfig.Issuer));
        string audience = jwtConfig.Audience ?? throw new ArgumentNullException(nameof(jwtConfig.Audience));

        SecurityTokenDescriptor tokenDescriptor = new()
        {
            Subject = new ClaimsIdentity(claims),
            Issuer = issuer,
            Audience = audience,
            Expires = DateTime.UtcNow.AddHours(jwtConfig.ValidFor),
            SigningCredentials = new SigningCredentials(new SymmetricSecurityKey(key), SecurityAlgorithms.HmacSha512Signature)
        };

        JwtSecurityTokenHandler tokenWriter = new();
        SecurityToken token = tokenWriter.CreateToken(tokenDescriptor);
        return tokenWriter.WriteToken(token);
    }
}