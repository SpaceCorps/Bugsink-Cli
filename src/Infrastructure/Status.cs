using Spectre.Console;

namespace Bugsink.Console.Infrastructure;

/// <summary>
/// Human-facing status and error lines. They go to stderr so that stdout stays parseable YAML even
/// for commands that both write a result and confirm what they did.
/// </summary>
public static class Status
{
    private static readonly IAnsiConsole Error = AnsiConsole.Create(new AnsiConsoleSettings
    {
        Out = new AnsiConsoleOutput(System.Console.Error)
    });

    public static void Ok(string message) => Error.MarkupLine($"[green]{message.EscapeMarkup()}[/]");

    public static void Fail(string message) => Error.MarkupLine($"[red]{message.EscapeMarkup()}[/]");
}
