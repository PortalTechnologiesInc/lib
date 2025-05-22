package cc.getportal.sdk

import cc.getportal.sdk.exception.PortalException
import cc.getportal.sdk.json.JsonUtils
import command.Command
import command.CommandWithId
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.WebSocket
import okhttp3.WebSocketListener
import org.slf4j.LoggerFactory
import java.util.*
import java.util.concurrent.ConcurrentHashMap
import java.util.concurrent.atomic.AtomicBoolean


class Portal(
    val hostAddress: String,
    val authToken: String,
    val nostrKey: String,
    val onClose: () -> Unit = {}
) {

    private val httpClient: OkHttpClient = OkHttpClient()
    private lateinit var socket: WebSocket
    private var authenticated = AtomicBoolean(false)

    private val commands: MutableMap<String, (Any) -> Unit> = ConcurrentHashMap()

    private var closed = AtomicBoolean(false)

    private fun getHealth(): Boolean {
        val request = Request.Builder()
            .url("$hostAddress/health")
            .addHeader("Authorization", "Bearer $authToken")
            .build()
        val response = httpClient.newCall(request).execute()

        val body = response.body ?: return false
        return body.string() == "OK"

    }

    private fun connect() {
        if (!getHealth()) {
            throw PortalException(message = "Server is not running")
        }
        startWsClient()
    }


    private fun startWsClient() {
        val request = Request.Builder()
            .url("$hostAddress/ws")
            .addHeader("Authorization", "Bearer $authToken")
            .build()

        socket = httpClient.newWebSocket(request, object : WebSocketListener() {
            override fun onMessage(webSocket: WebSocket, text: String) {
                // deserialize based on response
            }

            override fun onClosed(webSocket: WebSocket, code: Int, reason: String) {
                onClose()
            }
        })

        sendCommand(Command.Auth(token = authToken)) {
            authenticated.set(true)
        }
    }

    fun sendCommand(command: Command, task: (Any) -> Unit) {
        if(!authenticated.get()) {
            throw PortalException("Not authenticated")
        }
        val id = UUID.randomUUID().toString()
        socket.send(JsonUtils.serialize(CommandWithId(id = id, params = command)))
        commands[id] = task
    }


    companion object {
        private val logger = LoggerFactory.getLogger(Portal::class.java)
    }
}