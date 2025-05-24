package cc.getportal.sdk

import cc.getportal.sdk.command.Command
import cc.getportal.sdk.command.CommandWithId
import cc.getportal.sdk.command.Response
import cc.getportal.sdk.command.ResponseData
import cc.getportal.sdk.exception.PortalException
import cc.getportal.sdk.json.JsonUtils
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.WebSocket
import okhttp3.WebSocketListener
import org.slf4j.LoggerFactory
import java.net.SocketException
import java.util.*
import java.util.concurrent.ConcurrentHashMap
import java.util.concurrent.atomic.AtomicBoolean


class Portal(
    val hostAddress: String,
    val authToken: String,
    val onClose: (Int, String) -> Unit = { code, reason -> }
) {

    private data class InternalTask<R : ResponseData>(val onSuccess : (R) -> Unit, val onError: (String) -> Unit)

    private val httpClient: OkHttpClient = OkHttpClient()
    private lateinit var socket: WebSocket

    private val commands: MutableMap<String, InternalTask<ResponseData>> = ConcurrentHashMap()

    private var closed = AtomicBoolean(false)


    init {
        connect()
    }

    private fun getHealth(): Boolean {
        val request = Request.Builder()
            .url("$hostAddress/health")
            .addHeader("Authorization", "Bearer $authToken")
            .build()
        val response = httpClient.newCall(request).execute()

        val body = response.body ?: return false
        val bodyStr = body.string()
        response.close()
        return bodyStr == "OK"

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
                logger.info("Received {}", text)
                val response = JsonUtils.deserialize(text)
                when(response) {
                    is Response.Error -> {
                        commands.remove(response.id)?.onError?.invoke(response.message)
                    }
                    is Response.Notification -> {
                        TODO(reason = "Notification")
                    }
                    is Response.Success -> {
                        commands.remove(response.id)?.let { internalTask ->
                            internalTask.onSuccess.invoke(response.data)
                        }
                    }
                }
            }

            override fun onClosed(webSocket: WebSocket, code: Int, reason: String) {
                // not working on server closed
            }

            override fun onFailure(webSocket: WebSocket, t: Throwable, response: okhttp3.Response?) {

                if(t is SocketException && t.message == "Connection reset") {
                    closed.set(true)
                    onClose.invoke(1000, "Connection reset")
                }
            }

        })

        sendCommand(Command.Auth(token = authToken), onError = {
            logger.error("Authentication failed: {}", it)
        }) {
            // Authenticated
        }
    }

    fun <R : ResponseData> sendCommand(command: Command<R>, onError: (String) -> Unit, onSuccess: (R) -> Unit,) {
        if(closed.get()) {
            throw PortalException("Connection already closed")
        }


        val id = UUID.randomUUID().toString()
        val msg = JsonUtils.serialize(CommandWithId(id = id, params = command))
        logger.info("Sending {}", msg)
        socket.send(msg)
        commands[id] = InternalTask(onSuccess = onSuccess as (ResponseData) -> Unit, onError = onError)
    }


    companion object {
        private val logger = LoggerFactory.getLogger(Portal::class.java)
    }
}