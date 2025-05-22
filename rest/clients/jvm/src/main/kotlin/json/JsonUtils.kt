package cc.getportal.sdk.json

import com.fasterxml.jackson.databind.module.SimpleModule
import com.fasterxml.jackson.module.kotlin.jacksonObjectMapper
import command.CommandWithId
import command.CommandWithIdSerializer

object JsonUtils {

    val commandModule = SimpleModule().apply {
        addSerializer(CommandWithId::class.java, CommandWithIdSerializer())
    }

    val mapper = jacksonObjectMapper()
        .registerModule(commandModule)


    fun serialize(commandWithId : CommandWithId) : String = mapper.writeValueAsString(commandWithId)
}