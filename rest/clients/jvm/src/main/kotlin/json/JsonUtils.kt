package cc.getportal.sdk.json

import cc.getportal.sdk.command.CommandWithId
import cc.getportal.sdk.command.CommandWithIdSerializer
import cc.getportal.sdk.command.Response
import com.fasterxml.jackson.databind.module.SimpleModule
import com.fasterxml.jackson.module.kotlin.jacksonObjectMapper
import com.fasterxml.jackson.module.kotlin.readValue

object JsonUtils {

    val commandModule = SimpleModule().apply {
        addSerializer(CommandWithId::class.java, CommandWithIdSerializer())
    }

    val mapper = jacksonObjectMapper()
        .registerModule(commandModule)


    fun serialize(commandWithId : CommandWithId) : String = mapper.writeValueAsString(commandWithId)

    fun deserialize(text : String) : Response = mapper.readValue(text)
}