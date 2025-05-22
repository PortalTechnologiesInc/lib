package command

import com.fasterxml.jackson.annotation.*
import com.fasterxml.jackson.databind.annotation.JsonSerialize
import com.fasterxml.jackson.annotation.JsonInclude
import com.fasterxml.jackson.annotation.JsonProperty

@JsonSerialize(using = CommandWithIdSerializer::class)
data class CommandWithId(
    val id: String,
    val params: Command
) {
}

@JsonTypeInfo(use = JsonTypeInfo.Id.NAME, include = JsonTypeInfo.As.PROPERTY, property = "cmd")
@JsonSubTypes(
    JsonSubTypes.Type(value = Command.Auth::class, name = "Auth"),
    JsonSubTypes.Type(value = Command.NewAuthInitUrl::class, name = "NewAuthInitUrl"),
    JsonSubTypes.Type(value = Command.AuthenticateKey::class, name = "AuthenticateKey"),
    JsonSubTypes.Type(value = Command.RequestRecurringPayment::class, name = "RequestRecurringPayment"),
    JsonSubTypes.Type(value = Command.RequestSinglePayment::class, name = "RequestSinglePayment"),
    JsonSubTypes.Type(value = Command.RequestPaymentRaw::class, name = "RequestPaymentRaw"),
    JsonSubTypes.Type(value = Command.FetchProfile::class, name = "FetchProfile"),
    JsonSubTypes.Type(value = Command.SetProfile::class, name = "SetProfile")
)
sealed class Command {
    class NewAuthInitUrl : Command()

    data class Auth(val token: String) : Command()
    data class AuthenticateKey(val main_key: String, val subkeys: List<String>) : Command()

    @Deprecated(message = "Not completed yet")
    data class RequestRecurringPayment(
        val main_key: String,
        val subkeys: List<String>,
//        val payment_request: RecurringPaymentRequestContent
        val payment_request : Any = TODO()
    ) : Command()

    @Deprecated(message = "Not completed yet")
    data class RequestSinglePayment(
        val main_key: String,
        val subkeys: List<String>,
//        val payment_request: SinglePaymentParams
        val payment_request: Any = TODO()
    ) : Command()

    @Deprecated(message = "Not completed yet")
    data class RequestPaymentRaw(
        val main_key: String,
        val subkeys: List<String>,
//        val payment_request: SinglePaymentRequestContent
        val payment_request : Any = TODO()
    ) : Command()

    data class FetchProfile(val main_key: String) : Command()
    data class SetProfile(val profile: Profile) : Command()
}


@JsonInclude(JsonInclude.Include.NON_NULL)
data class Profile(
    val name: String? ,
    @JsonProperty("display_name")
    val displayName: String?,
    val picture: String?,
    val nip05: String?
)
