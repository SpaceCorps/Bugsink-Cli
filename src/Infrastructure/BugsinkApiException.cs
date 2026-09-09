namespace Bugsink.Console.Infrastructure;

public sealed class BugsinkApiException : Exception
{
    public BugsinkApiException(string message) : base(message) { }
}
