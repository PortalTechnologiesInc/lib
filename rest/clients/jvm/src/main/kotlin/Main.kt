package cc.getportal.sdk

import command.Command


fun main() {

    val portal = Portal(
        hostAddress = "http://localhost:2000",
        authToken = "authToken",
        nostrKey = "nostrKey"
    )

    portal.sendCommand(Command.NewAuthInitUrl, onError = {}) { (url, stream_id) ->
        // logic
    }

}