using System.Collections.Generic;
using System.Text.Json;
using System.Text.Json.Serialization;

namespace AWMKit.Native;

[JsonSourceGenerationOptions(JsonSerializerDefaults.Web)]
[JsonSerializable(typeof(List<AwmDatabaseBridge.TagMappingRow>))]
[JsonSerializable(typeof(List<AwmDatabaseBridge.EvidenceRow>))]
[JsonSerializable(typeof(List<AwmKeyBridge.KeySlotSummaryRow>))]
[JsonSerializable(typeof(string[]))]
[JsonSerializable(typeof(long[]))]
internal sealed partial class AwmJsonContext : JsonSerializerContext
{
}
