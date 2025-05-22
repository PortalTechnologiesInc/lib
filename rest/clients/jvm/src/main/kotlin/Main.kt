package cc.getportal.sdk

import command.Command
import command.CommandWithId
import java.util.UUID


fun main() {



    val portal = Portal(
        hostAddress = "localhost:2000",
        authToken = "authToken",
        nostrKey = "nostrKey"
    )

    portal.sendCommand(Command.NewAuthInitUrl()) {
        // on response
    }

}