package cc.getportal.sdk.command

import com.fasterxml.jackson.annotation.*
import cc.getportal.sdk.model.Profile

// ---------- CommandWithId wrapper ----------
data class CommandWithId(
    val id: String,
    val params: Command<*>
)


// ---------- Command base ----------
@JsonTypeInfo(use = JsonTypeInfo.Id.NAME, include = JsonTypeInfo.As.PROPERTY, property = "cmd")
@JsonSubTypes(
    JsonSubTypes.Type(Command.Auth::class, name = "Auth"),
    JsonSubTypes.Type(Command.NewAuthInitUrl::class, name = "NewAuthInitUrl"),
    JsonSubTypes.Type(Command.AuthenticateKey::class, name = "AuthenticateKey"),
    JsonSubTypes.Type(Command.RequestRecurringPayment::class, name = "RequestRecurringPayment"),
    JsonSubTypes.Type(Command.RequestSinglePayment::class, name = "RequestSinglePayment"),
    JsonSubTypes.Type(Command.RequestPaymentRaw::class, name = "RequestPaymentRaw"),
    JsonSubTypes.Type(Command.FetchProfile::class, name = "FetchProfile"),
    JsonSubTypes.Type(Command.SetProfile::class, name = "SetProfile")
)
sealed interface Command<R : ResponseData> {
    // --- Command implementations ---
    data class Auth(val token: String) : Command<ResponseData.AuthSuccess>
    data object NewAuthInitUrl : Command<ResponseData.AuthInitUrl>
    data class AuthenticateKey(val main_key: String, val subkeys: List<String>) : Command<ResponseData.AuthResponse>

    @Deprecated("Not fully implemented")
    data class RequestRecurringPayment(
        val main_key: String,
        val subkeys: List<String>,
        val payment_request: Any // TODO: Replace with actual type
    ) : Command<ResponseData.RecurringPayment>

    @Deprecated("Not fully implemented")
    data class RequestSinglePayment(
        val main_key: String,
        val subkeys: List<String>,
        val payment_request: Any // TODO: Replace with actual type
    ) : Command<ResponseData.SinglePayment>

    @Deprecated("Not fully implemented")
    data class RequestPaymentRaw(
        val main_key: String,
        val subkeys: List<String>,
        val payment_request: Any // TODO: Replace with actual type
    ) : Command<ResponseData.SinglePayment>

    data class FetchProfile(val main_key: String) : Command<ResponseData.ProfileData>
    data class SetProfile(val profile: Profile) : Command<ResponseData.ProfileData>
}



// ---------- Response sealed class ----------
@JsonTypeInfo(use = JsonTypeInfo.Id.NAME, include = JsonTypeInfo.As.PROPERTY, property = "type")
@JsonSubTypes(
    JsonSubTypes.Type(Response.Success::class, name = "success"),
    JsonSubTypes.Type(Response.Error::class, name = "error"),
    JsonSubTypes.Type(Response.Notification::class, name = "notification")
)
sealed class Response {
    data class Success(val id: String, val data: ResponseData) : Response()
    data class Error(val id: String, val message: String) : Response()
    data class Notification(val id: String, val data: NotificationData) : Response()
}

// ---------- ResponseData sealed class ----------
@JsonTypeInfo(use = JsonTypeInfo.Id.NAME, include = JsonTypeInfo.As.PROPERTY, property = "type")
@JsonSubTypes(
    JsonSubTypes.Type(ResponseData.AuthSuccess::class, name = "auth_success"),
    JsonSubTypes.Type(ResponseData.AuthInitUrl::class, name = "auth_init_url"),
    JsonSubTypes.Type(ResponseData.AuthResponse::class, name = "auth_response"),
    JsonSubTypes.Type(ResponseData.RecurringPayment::class, name = "recurring_payment"),
    JsonSubTypes.Type(ResponseData.SinglePayment::class, name = "single_payment"),
    JsonSubTypes.Type(ResponseData.ProfileData::class, name = "profile")
)
sealed class ResponseData {
    data class AuthSuccess(val message: String) : ResponseData()
    data class AuthInitUrl(val url: String, val stream_id: String) : ResponseData()
    data class AuthResponse(val event: AuthResponseData) : ResponseData()
    @Deprecated("Not fully implemented")
    data class RecurringPayment(val status: Any) : ResponseData() // TODO: Replace Any
    @Deprecated("Not fully implemented")
    data class SinglePayment(val status: Any, val stream_id: String?) : ResponseData() // TODO: Replace Any
    data class ProfileData(val profile: Profile?) : ResponseData()
}

data class AuthResponseData(
    val user_key: String,
    val recipient: String,
    val challenge: String,
    val granted_permissions: List<String>,
    val session_token: String
)

// ---------- NotificationData sealed class ----------
sealed class NotificationData {
    data class AuthInit(val main_key: String) : NotificationData()
    data class PaymentStatusUpdate(val status: InvoiceStatus) : NotificationData()
}

sealed class InvoiceStatus {
    data class Paid(val preimage: String?) : InvoiceStatus()
    object Timeout : InvoiceStatus()
    data class Error(val reason: String) : InvoiceStatus()
}