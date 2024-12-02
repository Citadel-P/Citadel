namespace Application.Utils;

public static class Constants
{

    public static string Url => "[(http(s)?):\\/\\/(www\\.)?a-zA-Z0-9@:%._\\+~#=]{2,256}\\.[a-z]{2,6}\\b([-a-zA-Z0-9@:%_\\+.~#?&//=]*)";

    public const string JwtFilePath = "data/jwtsecret";
}