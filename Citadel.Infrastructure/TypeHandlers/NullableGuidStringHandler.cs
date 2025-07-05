using System.Data;
using Dapper;

namespace Infrastructure.TypeHandlers;

public sealed class NullableGuidStringHandler : SqlMapper.TypeHandler<Guid?>
{
    public override Guid? Parse(object value)
    {
        return value is string str && Guid.TryParse(str, out var guid)
            ? guid
            : null;
    }

    public override void SetValue(IDbDataParameter parameter, Guid? value)
    {
        if (value is Guid g)
        {
            Span<char> buffer = stackalloc char[36];
            if (g.TryFormat(buffer, out int charsWritten))
                parameter.Value = buffer[..charsWritten].ToString();
            else
                parameter.Value = g.ToString();
        }
        else
        {
            parameter.Value = DBNull.Value;
        }
    }
}
