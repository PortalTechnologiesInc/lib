import cc.getportal.sdk.Portal
import cc.getportal.sdk.command.Command
import cc.getportal.sdk.model.Profile
import kotlin.system.exitProcess


fun main() {

    val portal = Portal(
        hostAddress = "http://127.0.0.1:3000",
        authToken = "test",
        onClose = { code, reason ->
            System.err.println("Connection closed($code): $reason")
            exitProcess(code)
        }
    )

    portal.sendCommand(Command.NewAuthInitUrl, onError = {}) { (url, stream_id) ->
        // logic
    }

    val profile = Profile(
        name = "test",
        displayName = "Test",
        picture = null,
        nip05 = null
    )
    portal.sendCommand(Command.SetProfile(profile), onError = {}) { profileData ->
        val p = profileData.profile
        // logic
    }

    portal.sendCommand(Command.RequestPaymentRaw("", emptyList(), Any()), onError = {}) {

    }


}