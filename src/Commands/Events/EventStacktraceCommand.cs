using System.ComponentModel;
using Bugsink.Console.Infrastructure;
using Spectre.Console.Cli;

namespace Bugsink.Console.Commands.Events;

public sealed class EventStacktraceCommand : AsyncCommand<EventStacktraceCommand.Settings>
{
    public sealed class Settings : ApiSettings
    {
        [CommandArgument(0, "<EVENT>")]
        [Description("Bugsink-internal event UUID, as listed by 'events list'")]
        public required string Event { get; init; }
    }

    public override async Task<int> ExecuteAsync(CommandContext context, Settings settings)
    {
        var client = settings.CreateClient();
        var path = $"events/{Uri.EscapeDataString(settings.Event)}/stacktrace/";
        System.Console.WriteLine((await client.GetTextAsync(path)).TrimEnd());
        return 0;
    }
}
