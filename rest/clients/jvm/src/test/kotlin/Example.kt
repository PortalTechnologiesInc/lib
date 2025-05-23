import cc.getportal.sdk.Portal
import cc.getportal.sdk.command.Command
import cc.getportal.sdk.model.Profile


fun main() {

    val portal = Portal(
        hostAddress = "http://localhost:2000",
        authToken = "authToken",
        nostrKey = "nostrKey"
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

}