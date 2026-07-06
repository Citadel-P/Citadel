namespace Domain.Entities.Oidc;

public sealed class OidcProvider(
    string name,
    string? description,
    string displayName,
    string issuer,
    string clientId,
    string? clientSecretCiphertext,
    string scopes,
    bool enabled,
    bool autoProvisionUsers,
    bool allowEmailAutoLink,
    bool requireEmailVerified,
    string? allowedEmailDomains,
    string? requiredClaimName,
    string? requiredClaimValues,
    Guid? defaultRoleId,
    Guid createdByActorId,
    DateTime? createdAt = null,
    DateTime? updatedAt = null) : IAuditedEntity
{
    public Guid Id { get; private set; } = Guid.CreateVersion7();
    public string Name { get; private set; } = name;
    public string? Description { get; private set; } = NormalizeOptional(description);
    public string DisplayName { get; private set; } = displayName;
    public string Issuer { get; private set; } = NormalizeIssuer(issuer);
    public string ClientId { get; private set; } = clientId;
    public string? ClientSecretCiphertext { get; private set; } = clientSecretCiphertext;
    public string Scopes { get; private set; } = NormalizeScopes(scopes);
    public bool Enabled { get; private set; } = enabled;
    public bool AutoProvisionUsers { get; private set; } = autoProvisionUsers;
    public bool AllowEmailAutoLink { get; private set; } = allowEmailAutoLink;
    public bool RequireEmailVerified { get; private set; } = requireEmailVerified;
    public string? AllowedEmailDomains { get; private set; } = NormalizeOptionalCsv(allowedEmailDomains);
    public string? RequiredClaimName { get; private set; } = NormalizeOptional(requiredClaimName);
    public string? RequiredClaimValues { get; private set; } = NormalizeOptionalCsv(requiredClaimValues);
    public Guid? DefaultRoleId { get; private set; } = defaultRoleId;
    public DateTime CreatedAt { get; private set; } = createdAt ?? DateTime.UtcNow;
    public Guid CreatedByActorId { get; private set; } = createdByActorId;
    public DateTime UpdatedAt { get; private set; } = updatedAt ?? DateTime.UtcNow;

    public void Rename(string name)
    {
        Name = name;
        UpdatedAt = DateTime.UtcNow;
    }

    public void UpdateDescription(string? description)
    {
        Description = NormalizeOptional(description);
        UpdatedAt = DateTime.UtcNow;
    }

    public void Update(
        string? name,
        string? description,
        bool updateDescription,
        string? displayName,
        string? issuer,
        string? clientId,
        string? clientSecretCiphertext,
        string? scopes,
        bool? enabled,
        bool? autoProvisionUsers,
        bool? allowEmailAutoLink,
        bool? requireEmailVerified,
        string? allowedEmailDomains,
        bool updateAllowedEmailDomains,
        string? requiredClaimName,
        bool updateRequiredClaimName,
        string? requiredClaimValues,
        bool updateRequiredClaimValues,
        Guid? defaultRoleId,
        bool updateDefaultRoleId)
    {
        if (name is not null)
            Name = name;

        if (updateDescription)
            Description = NormalizeOptional(description);

        if (displayName is not null)
            DisplayName = displayName;

        if (issuer is not null)
            Issuer = NormalizeIssuer(issuer);

        if (clientId is not null)
            ClientId = clientId;

        if (clientSecretCiphertext is not null)
            ClientSecretCiphertext = clientSecretCiphertext;

        if (scopes is not null)
            Scopes = NormalizeScopes(scopes);

        if (enabled.HasValue)
            Enabled = enabled.Value;

        if (autoProvisionUsers.HasValue)
            AutoProvisionUsers = autoProvisionUsers.Value;

        if (allowEmailAutoLink.HasValue)
            AllowEmailAutoLink = allowEmailAutoLink.Value;

        if (requireEmailVerified.HasValue)
            RequireEmailVerified = requireEmailVerified.Value;

        if (updateAllowedEmailDomains)
            AllowedEmailDomains = NormalizeOptionalCsv(allowedEmailDomains);

        if (updateRequiredClaimName)
            RequiredClaimName = NormalizeOptional(requiredClaimName);

        if (updateRequiredClaimValues)
            RequiredClaimValues = NormalizeOptionalCsv(requiredClaimValues);

        if (updateDefaultRoleId)
            DefaultRoleId = defaultRoleId == Guid.Empty ? null : defaultRoleId;

        UpdatedAt = DateTime.UtcNow;
    }

    public void Validate()
    {
        if (string.IsNullOrWhiteSpace(Name))
            throw new ArgumentException("OIDC provider name is required.", nameof(Name));

        if (Name.Length > 128)
            throw new ArgumentException("OIDC provider name cannot exceed 128 characters.", nameof(Name));

        if (Description?.Length > 600)
            throw new ArgumentException("OIDC provider description cannot exceed 600 characters.", nameof(Description));

        if (string.IsNullOrWhiteSpace(DisplayName))
            throw new ArgumentException("OIDC provider display name is required.", nameof(DisplayName));

        if (DisplayName.Length > 128)
            throw new ArgumentException("OIDC provider display name cannot exceed 128 characters.", nameof(DisplayName));

        if (!Uri.TryCreate(Issuer, UriKind.Absolute, out var issuerUri)
            || issuerUri.Scheme is not "https" and not "http")
        {
            throw new ArgumentException("OIDC provider issuer must be an absolute HTTP or HTTPS URL.", nameof(Issuer));
        }

        if (string.IsNullOrWhiteSpace(ClientId))
            throw new ArgumentException("OIDC provider client id is required.", nameof(ClientId));

        if (ClientId.Length > 256)
            throw new ArgumentException("OIDC provider client id cannot exceed 256 characters.", nameof(ClientId));

        if (string.IsNullOrWhiteSpace(Scopes))
            throw new ArgumentException("OIDC provider scopes are required.", nameof(Scopes));

        if (!Scopes.Split(' ', StringSplitOptions.RemoveEmptyEntries).Contains("openid", StringComparer.Ordinal))
            throw new ArgumentException("OIDC provider scopes must include openid.", nameof(Scopes));

        if (AllowedEmailDomains is not null)
        {
            foreach (var domain in SplitCsv(AllowedEmailDomains))
            {
                if (!domain.Contains('.', StringComparison.Ordinal) || domain.Contains('@', StringComparison.Ordinal))
                    throw new ArgumentException("Allowed email domains must be plain domains such as example.com.", nameof(AllowedEmailDomains));
            }
        }

        if (!string.IsNullOrWhiteSpace(RequiredClaimName) && string.IsNullOrWhiteSpace(RequiredClaimValues))
            throw new ArgumentException("Required claim values are required when a required claim name is configured.", nameof(RequiredClaimValues));
    }

    public static OidcProvider FromPersistence(
        Guid id,
        string name,
        string? description,
        string displayName,
        string issuer,
        string clientId,
        string? clientSecretCiphertext,
        string scopes,
        bool enabled,
        bool autoProvisionUsers,
        bool allowEmailAutoLink,
        bool requireEmailVerified,
        string? allowedEmailDomains,
        string? requiredClaimName,
        string? requiredClaimValues,
        Guid? defaultRoleId,
        Guid createdByActorId,
        DateTime createdAt,
        DateTime updatedAt)
    {
        return new OidcProvider(
            name,
            description,
            displayName,
            issuer,
            clientId,
            clientSecretCiphertext,
            scopes,
            enabled,
            autoProvisionUsers,
            allowEmailAutoLink,
            requireEmailVerified,
            allowedEmailDomains,
            requiredClaimName,
            requiredClaimValues,
            defaultRoleId,
            createdByActorId,
            createdAt,
            updatedAt)
        {
            Id = id
        };
    }

    private static string NormalizeIssuer(string issuer)
        => issuer.Trim().TrimEnd('/');

    private static string NormalizeScopes(string scopes)
        => string.Join(' ', scopes.Split(' ', StringSplitOptions.RemoveEmptyEntries | StringSplitOptions.TrimEntries));

    private static string? NormalizeOptional(string? value)
        => string.IsNullOrWhiteSpace(value) ? null : value.Trim();

    private static string? NormalizeOptionalCsv(string? value)
        => string.IsNullOrWhiteSpace(value)
            ? null
            : string.Join(',', SplitCsv(value));

    private static string[] SplitCsv(string value)
        => value
            .Split(',', StringSplitOptions.RemoveEmptyEntries | StringSplitOptions.TrimEntries)
            .Where(static item => !string.IsNullOrWhiteSpace(item))
            .Distinct(StringComparer.OrdinalIgnoreCase)
            .ToArray();
}
