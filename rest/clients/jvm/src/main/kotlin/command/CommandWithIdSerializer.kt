package command

import com.fasterxml.jackson.core.JsonGenerator
import com.fasterxml.jackson.databind.JsonSerializer
import com.fasterxml.jackson.databind.SerializerProvider
import com.fasterxml.jackson.databind.ObjectMapper
import com.fasterxml.jackson.module.kotlin.jacksonObjectMapper
import com.fasterxml.jackson.module.kotlin.registerKotlinModule

class CommandWithIdSerializer : JsonSerializer<CommandWithId>() {
    private val internalMapper = jacksonObjectMapper()

    override fun serialize(
        value: CommandWithId,
        gen: JsonGenerator,
        serializers: SerializerProvider
    ) {
        gen.writeStartObject()
        gen.writeStringField("id", value.id)
        val cmdName = value.params::class.simpleName ?: "Unknown"
        gen.writeStringField("cmd", cmdName)
        gen.writeFieldName("params")
        internalMapper.writeValue(gen, value.params)
        gen.writeEndObject()
    }
}