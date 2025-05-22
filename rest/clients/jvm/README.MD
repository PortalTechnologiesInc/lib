# Portal SDK (JVM)

A Kotlin SDK for interacting with the Portal service via WebSocket and REST API.
This library targets the **JVM platform** in general and can be used not only from Kotlin but also from **Java** or any other JVM-based language.

It provides a typed command/response system using sealed classes for safe and clear protocol handling.

---

## Features

* Typed commands modeled as `sealed interface Command<R : ResponseData>`
* Typed responses using `sealed class Response` and `ResponseData`
* WebSocket client with authentication and asynchronous command handling
* JSON serialization/deserialization with Jackson
* Support for notifications and payment/status updates (extendable)
* JVM-compatible, usable from Kotlin, Java, Scala, Groovy, and other JVM languages

---

## Installation

**Work in progress** – The library is currently under development and not yet published to any package repository.
Installation instructions will be provided soon.

---

## Usage Example

```kotlin
import cc.getportal.sdk.Portal
import cc.getportal.sdk.command.Command
import cc.getportal.sdk.model.Profile

fun main() {
    val portal = Portal(
        hostAddress = "http://localhost:2000",
        authToken = "authToken",
        nostrKey = "nostrKey"
    )

    // Request new authentication init URL
    portal.sendCommand(Command.NewAuthInitUrl, 
      onError = { error -> println("Error: $error") }) { response ->
        val url = response.url
        val streamId = response.stream_id
        println("Auth Init URL: $url, Stream ID: $streamId")
    }

    // Update profile
    val profile = Profile(
        name = "test",
        displayName = "Test",
        picture = null,
        nip05 = null
    )

    portal.sendCommand(Command.SetProfile(profile), onError = {}) { profileData ->
        val updatedProfile = profileData.profile
        println("Updated Profile: $updatedProfile")
    }
}
```

---

## Contributing

Feel free to open issues or pull requests for bugs, features, or improvements.
