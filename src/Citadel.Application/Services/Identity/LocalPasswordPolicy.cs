using System.Text;

namespace Application.Services.Identity;

public static class LocalPasswordPolicy
{
    public const int MinimumLength = 15;
    public const int MaximumLength = 128;

    private static readonly HashSet<string> BlockedPasswords = new(StringComparer.OrdinalIgnoreCase)
    {
        "admin",
        "admin123",
        "password",
        "password123",
        "letmein",
        "citadel",
        "citadel123"
    };

    public static string? GetValidationError(
        string? password,
        string? userName = null,
        string? email = null)
    {
        if (password is null)
            return "Password is required.";

        var length = password.EnumerateRunes().Count();
        if (length < MinimumLength)
            return $"Password must be at least {MinimumLength} characters.";

        if (length > MaximumLength)
            return $"Password must be no more than {MaximumLength} characters.";

        if (BlockedPasswords.Contains(password))
            return "Choose a less common password.";

        if (!string.IsNullOrEmpty(userName)
            && string.Equals(password, userName, StringComparison.OrdinalIgnoreCase))
        {
            return "Password must not match the username.";
        }

        if (!string.IsNullOrEmpty(email))
        {
            if (string.Equals(password, email, StringComparison.OrdinalIgnoreCase))
                return "Password must not match the email address.";

            var separator = email.IndexOf('@');
            if (separator > 0
                && string.Equals(password, email[..separator], StringComparison.OrdinalIgnoreCase))
            {
                return "Password must not match the email address.";
            }
        }

        return null;
    }
}
