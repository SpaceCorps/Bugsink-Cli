using YamlDotNet.Serialization;
using YamlDotNet.Serialization.NamingConventions;

namespace Bugsink.Console.Infrastructure;

public static class YamlOutput
{
    private static readonly ISerializer Serializer = new SerializerBuilder()
        .WithNamingConvention(NullNamingConvention.Instance)
        .Build();

    public static void Write(object? value)
    {
        if (value is null)
        {
            System.Console.WriteLine("null");
            return;
        }

        if (value is string or bool or long or int or double)
        {
            System.Console.WriteLine(value is bool b
                ? (b ? "true" : "false")
                : Convert.ToString(value, System.Globalization.CultureInfo.InvariantCulture));
            return;
        }

        System.Console.WriteLine(Serializer.Serialize(value).TrimEnd());
    }
}
