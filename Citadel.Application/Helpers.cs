using System.Security.Claims;
using System.Security.Cryptography;

namespace Application;

public static class Helpers
{
    /// <summary>
    /// Generate a random password
    /// </summary>
    public static string GetRandomPassword()
    {
        var salt = new byte[94];

        using (var csprng = RandomNumberGenerator.Create())
        {
            csprng.GetBytes(salt);
        }
        return Convert.ToBase64String(salt);
    }

    /// <summary>
    /// Gets the jwt signing key from the specified file
    /// </summary>
    /// <remarks>The jwt key is stored in the volume with no encryption</remarks>
    public static string GetJwtSecretFromFile(string jwtFilePath = Constants.JwtFilePath)
    {
        string jwtKey;
        if (File.Exists(jwtFilePath))
        {
            char[] buffer = new char[128];
            using StreamReader reader = new(jwtFilePath);

            int bytesRead = reader.Read(buffer, 0, buffer.Length);
            jwtKey = new string(buffer, 0, bytesRead);
        }
        else
        {
            jwtKey = GetRandomPassword();
            using StreamWriter writer = new(jwtFilePath);
            writer.Write(jwtKey);
        }

        return jwtKey;
    }

    /// <summary>
    /// Gets the user id of the requester
    /// </summary>
    public static int GetUserId(this ClaimsPrincipal caller)
    {
        if (int.TryParse(caller.Claims.FirstOrDefault(c => c.Type == "userId")?.Value, out int result))
        {
            return result;
        }
        return result;
    }
}