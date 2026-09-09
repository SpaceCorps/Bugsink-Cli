using System.ComponentModel;
using Bugsink.Console.Infrastructure;
using Spectre.Console.Cli;

namespace Bugsink.Console.Commands.Events;

public sealed class GetEventCommand : AsyncCommand<GetEventCommand.Settings>
{
    public sealed class Settings : ApiSettings
    {
        [CommandArgument(0, "<EVENT>")]
        [Description("Bugsink-internal event UUID, as listed by 'events list'")]
        public required string Event { get; init; }

        [CommandOption("--no-data")]
        [Description("Omit the full event payload, which is large and mostly SDK metadata")]
        public bool NoData { get; init; }
    }

    public override async Task<int> ExecuteAsync(CommandContext context, Settings settings)
    {
        var client = settings.CreateClient();
        var result = await client.GetAsync($"events/{Uri.EscapeDataString(settings.Event)}/");

        if (settings.NoData && result is Dictionary<string, object?> row)
            row.Remove("data");

        YamlOutput.Write(result);
        return 0;
    }
}
