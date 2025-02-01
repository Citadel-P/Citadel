namespace Application.Configs;

public class JwtConfig
{
    /// <summary>
    /// 4.1.1.  "iss" (Issuer) Claim - The "iss" (issuer) claim identifies the principal that issued the JWT.
    /// </summary>
    public string Issuer { get; set; }

    /// <summary>
    /// 4.1.3.  "aud" (Audience) Claim - The "aud" (audience) claim identifies the recipients that the JWT is intended for.
    /// </summary>
    public string Audience { get; set; }

    /// <summary>
    /// Set the timespan the token will be valid for (in hours)
    /// </summary>
    public double ValidFor { get; set; } = 8;

    /// <summary>
    /// Secret key to generate the jwt signing key
    /// </summary>
    public string Key { get; set; }
}