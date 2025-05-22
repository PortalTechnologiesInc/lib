package model

import com.fasterxml.jackson.annotation.JsonInclude
import com.fasterxml.jackson.annotation.JsonProperty

@JsonInclude(JsonInclude.Include.NON_NULL)
data class Profile(
    val name: String?,
    @JsonProperty("display_name") val displayName: String?,
    val picture: String?,
    val nip05: String?
)