package cc.getportal.sdk.command

import com.fasterxml.jackson.core.JsonGenerator
import com.fasterxml.jackson.databind.JsonSerializer
import com.fasterxml.jackson.databind.SerializerProvider
import com.fasterxml.jackson.databind.ObjectMapper

class CommandWithIdSerializer : JsonSerializer<CommandWithId>() {
//    private val internalMapper = jacksonObjectMapper().registerKotlinModule()

    override fun serialize(
        value: CommandWithId,
        gen: JsonGenerator,
        serializers: SerializerProvider
    ) {
        val objectMapper = gen.codec as ObjectMapper
        val paramsTree = objectMapper.valueToTree<com.fasterxml.jackson.databind.node.ObjectNode>(value.params)

        val cmd = paramsTree.remove("cmd")?.asText()
            ?: value.params::class.simpleName
            ?: "Unknown"

        gen.writeStartObject()
        gen.writeStringField("id", value.id)
        gen.writeStringField("cmd", cmd)

        // fix serde deserialization of unit types like NewAuthInitUrl
        if (!paramsTree.isEmpty) {
            gen.writeFieldName("params")
            objectMapper.writeTree(gen, paramsTree)
        }

        gen.writeEndObject()
    }
}
