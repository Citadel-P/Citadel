namespace Application.Configs;

public class JwtConfig
{
    /// <summary>
    /// 4.1.1.  "iss" (Issuer) Claim - The "iss" (issuer) claim identifies the principal that issued the JWT.
    /// </summary>
    required public string Issuer { get; set; }

    /// <summary>
    /// 4.1.3.  "aud" (Audience) Claim - The "aud" (audience) claim identifies the recipients that the JWT is intended for.
    /// </summary>
    required public string Audience { get; set; }

    /// <summary>
    /// Access token configuration
    /// </summary>
    required public AccessToken AccessToken { get; set; }

    /// <summary>
    /// Refresh token configuration
    /// </summary>
    required public RefreshToken RefreshToken { get; set; }

    /// <summary>
    /// Secret key to generate the jwt signing key
    /// </summary>
    required public string Key { get; set; }
}

public record AccessToken(int ValidForMinutes = 15);
public record RefreshToken(int ValidForDays = 30);